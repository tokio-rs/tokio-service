use std::marker::PhantomData;

use Service;

/// Often, many of the pieces needed for writing network applications
/// can be reused across multiple services. The `Middleware` trait can
/// be used to write reusable components that can be applied to very
/// different kinds of services; for example, it can be applied to
/// services operating on different protocols, and to both the client
/// and server side of a network transaction.
///
/// # Timeouts
///
/// Take timeouts as an example:
///
/// ```rust,ignore
/// use tokio::Service;
/// use tokio::Middleware;
/// use futures::Future;
/// use std::time::Duration;
///
/// // Not yet implemented, but soon :)
/// use tokio::timer::{Timer, Expired};
///
///
/// pub struct Timeout {
///     delay: Duration,
///     timer: Timer,
/// }
///
/// impl Timeout {
///     fn timeout(&self) -> impl Future<Item = (), Error = Expired> {
///         self.timer.timeout(self.delay)
///     }
/// }
/// 
/// impl<S> Middleware<S> for Timeout
///     where S: Service,
///           S::Error: From<Expired>,
/// {
///     type WrappedService = TimeoutService<S>;
///     
///     fn wrap(self, upstream: S) -> TimeoutService<S> {
///         TimeoutService { timeout: self, upstream }
///     }
/// }
///
///
/// // This service implements the Timeout behavior.
/// pub struct TimeoutService<S> {
///     upstream: S,
///     timeout: Timeout,
/// }
///
/// impl<S> Service for TimeoutService<S>
///     where S: Service,
///           S::Error: From<Expired>,
/// {
///     type Request = S::Request;
///     type Response = S::Response;
///     type Error = S::Error;
///     type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
///
///     fn call(&self, req: Self::Req) -> Self::Future {
///         let timeout = self.timeout.timeout()
///             .and_then(|timeout| Err(Self::Error::from(timeout)));
///
///         self.upstream.call(req)
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
pub trait Middleware<S: Service> {
    /// The service produced by wrapping this middleware around another
    /// service.
    type WrappedService: Service;

    /// Wrap the middlware around a Service it is able to wrap.
    ///
    /// This produces a service of the `WrappedService` associated
    /// type, which itself is another service that could possibly be
    /// wrapped in other middleware.
    fn wrap(self, service: S) -> Self::WrappedService;

    /// Chain two middleware together. The lefthand side of this
    /// operation is the "inner" middleware and the righthand side is
    /// the "outer" middleware.
    ///
    /// When wrapping a middleware chain around a service, first the
    /// inner middleware is wrapped around that service, and then the
    /// outer middleware is wrapped around the service produced by the
    /// inner middleware.
    ///
    /// This allows you to build middleware chains before knowing
    /// exactly which service that chain applies to.
    fn chain<M>(self, middleware: M) -> MiddlewareChain<S, Self, M> where
        M: Middleware<Self::WrappedService>,
        Self: Sized,
    {
        MiddlewareChain {
            inner_middleware: self,
            outer_middleware: middleware,
            _marker: PhantomData,
        }
    }
}

/// Two middleware, chained together. This type is produced by the
/// `chain` method on the Middleware trait.
pub struct MiddlewareChain<S, InnerM, OuterM>
    where S: Service,
          InnerM: Middleware<S>,
          OuterM: Middleware<InnerM::WrappedService>,
{
    inner_middleware: InnerM,
    outer_middleware: OuterM,
    _marker: PhantomData<S>,
}

impl<S, InnerM, OuterM> Middleware<S> for MiddlewareChain<S, InnerM, OuterM>
    where S: Service,
          InnerM: Middleware<S>,
          OuterM: Middleware<InnerM::WrappedService>,
{
    type WrappedService = OuterM::WrappedService;

    fn wrap(self, service: S) -> Self::WrappedService {
        service.wrap(self.inner_middleware).wrap(self.outer_middleware)
    }
}
