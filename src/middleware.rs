use std::io;
use std::rc::Rc;
use std::sync::Arc;

use futures::Future;

use {Service, NewService};

/// A middleware wrapper around a Service.
///
/// More often than not, many of the pieces needed for writing robust, scalable
/// network applications are the same no matter the underlying protocol. By
/// unifying the API for both clients and servers in a protocol agnostic way,
/// it is possible to write middleware that provide these pieces in a
/// reusable way.
///
/// For example, take timeouts as an example:
///
/// ```rust,ignore
/// use tokio::{Service, Middleware};
/// use futures::Future;
/// use std::time::Duration;
/// use std::sync::Arc;
///
/// // Not yet implemented, but soon :)
/// use tokio::timer::{Timer, Expired};
///
/// pub struct Timeout<T> {
///     delay: Duration,
///     timer: Timer,
/// }
///
/// impl<T> Timeout<T> {
///     pub fn new(delay: Duration) -> Timeout<T> {
///         Timeout {
///             delay: delay,
///             timer: Timer::default(),
///         }
///     }
/// }
///
/// impl<T> Middleware<T> for Timeout<T>
///     where T: Service,
///           T::Error: From<Expired>,
/// {
///     type Request = T::Request;
///     type Response = T::Response;
///     type Error = T::Error;
///     type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
///
///     fn call(&self, req: Self::Req, service: &Arc<T>) -> Self::Future {
///         let timeout = self.timer.timeout(self.delay)
///             .and_then(|timeout| Err(Self::Error::from(timeout)));
///
///         service.call(req)
///             .select(timeout)
///             .map(|(v, _)| v)
///             .map_err(|(e, _)| e)
///             .boxed()
///     }
/// }
///
/// ```
///
/// The above timeout implementation is decoupled from the underlying protocol
/// and is also decoupled from client or server concerns. In other words, the
/// same timeout middleware could be used in either a client or a server.
pub trait Middleware<S: Service + ?Sized> {
    /// Requests handled by the middleware.
    type Request;

    /// Responses given by the middleware.
    type Response;

    /// Errors produced by the middleware.
    type Error;

    /// The future response value.
    type Future: Future<Item = Self::Response, Error = Self::Error>;

    /// Process the request and return the response asynchronously.
    ///
    /// This method receives a reference to the interior service that it is wrapping.
    fn call(&self, req: Self::Request, service: &Arc<S>) -> Self::Future;

    /// Wrap a service with this middleware.
    fn wrap(self, service: Arc<S>) -> WrappedService<S, Self> where Self: Sized {
        WrappedService::new(service, self)
    }
}

/// Create a new `Middleware` values.
pub trait NewMiddleware<S: NewService + ?Sized> {
    /// Requests handled by the middleware.
    type Request;

    /// Responses given by the middleware.
    type Response;

    /// Errors produced by the middleware.
    type Error;

    /// The `Middleware` value created by this factory
    type Instance: Middleware<S::Instance, Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    /// Create and return a new middleware value.
    fn new_middleware(&self) -> io::Result<Self::Instance>;

    /// Wrap a service factory with this middleware factory.
    fn wrap(self, service_factory: S) -> ServiceWrapper<S, Self> where
        S: Sized,
        Self: Sized,
    {
        ServiceWrapper::new(service_factory, self)
    }
}

/// A WrappedService is a Service wrapped in a Middleware. It can be
/// constructed using the Service::wrap method.
pub struct WrappedService<S: Service + ?Sized, M: Middleware<S>> {
    service: Arc<S>,
    middleware: M,
}

impl<S: ?Sized, M> Service for WrappedService<S, M> where
    S: Service,
    M: Middleware<S>,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Future = M::Future;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.middleware.call(req, &self.service)
    }
}

impl<S: ?Sized, M> WrappedService<S, M> where
    S: Service,
    M: Middleware<S>,
{
    /// Construct a new WrappedService from a Service and a Middleware.
    pub fn new(service: Arc<S>, middleware: M) -> WrappedService<S, M> {
        WrappedService {
            service: service,
            middleware: middleware,
        }
    }
}

/// A ServiceWrapper is a factory that constructs a service wrapped with a middleware.
/// It can be constructed with the NewService::wrap method.
pub struct ServiceWrapper<S: NewService, M: NewMiddleware<S>> {
    service_factory: S,
    middleware_factory: M,
}

impl<S, M> NewService for ServiceWrapper<S, M> where
    S: NewService,
    M: NewMiddleware<S>,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Instance = WrappedService<S::Instance, M::Instance>;

    fn new_service(&self) -> io::Result<Self::Instance> {
        let service = self.service_factory.new_service()?;
        let middleware = self.middleware_factory.new_middleware()?;
        Ok(WrappedService::new(Arc::new(service), middleware))
    }
}

impl<S, M> ServiceWrapper<S, M> where
    S: NewService,
    M: NewMiddleware<S>,
{
    /// Construct a new ServiceWrapper from a NewService and a NewMiddleware.
    pub fn new(service_factory: S, middleware_factory: M) -> ServiceWrapper<S, M> {
        ServiceWrapper {
            service_factory: service_factory,
            middleware_factory: middleware_factory,
        }
    }
}

impl<F, S, R> NewMiddleware<S> for F where
    F: Fn() -> io::Result<R>,
    S: NewService,
    R: Middleware<S::Instance>
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;

    fn new_middleware(&self) -> io::Result<R> {
        (*self)()
    }
}

impl<M: ?Sized, S: ?Sized> NewMiddleware<S> for Arc<M> where
    M: NewMiddleware<S>,
    S: NewService,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Instance = M::Instance;

    fn new_middleware(&self) -> io::Result<M::Instance> {
        (**self).new_middleware()
    }
}

impl<M: ?Sized, S: ?Sized> NewMiddleware<S> for Rc<M> where
    M: NewMiddleware<S>,
    S: NewService,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Instance = M::Instance;

    fn new_middleware(&self) -> io::Result<M::Instance> {
        (**self).new_middleware()
    }
}

impl<M: ?Sized, S: ?Sized> Middleware<S> for Box<M> where
    M: Middleware<S>,
    S: Service,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Future = M::Future;

    fn call(&self, req: Self::Request, service: &Arc<S>) -> Self::Future {
        (**self).call(req, service)
    }
}

impl<M: ?Sized, S: ?Sized> Middleware<S> for Rc<M> where
    M: Middleware<S>,
    S: Service,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Future = M::Future;

    fn call(&self, req: Self::Request, service: &Arc<S>) -> Self::Future {
        (**self).call(req, service)
    }
}

impl<M: ?Sized, S: ?Sized> Middleware<S> for Arc<M> where
    M: Middleware<S>,
    S: Service,
{
    type Request = M::Request;
    type Response = M::Response;
    type Error = M::Error;
    type Future = M::Future;

    fn call(&self, req: Self::Request, service: &Arc<S>) -> Self::Future {
        (**self).call(req, service)
    }
}
