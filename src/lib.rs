//! Definition of the core `Service` trait to Tokio
//!
//! More information can be found on [the trait] itself.
//!
//! [the trait]: trait.Service.html

#![deny(missing_docs)]

extern crate futures;

use futures::Future;

use std::io;

/// An asynchronous function from `Request` to a `Response`.
///
/// The `Service` trait is a simplified interface making it easy to write
/// network applications in a modular and reusable way, decoupled from the
/// underlying protocol. It is one of Tokio's fundamental abstractions.
///
/// # Functional
///
/// A `Service` is a function from a `Request`. It immediately returns a
/// `Future` representing the eventual completion of processing the
/// request. The actual request processing may happen at any time in the
/// future, on any thread or executor. The processing may depend on calling
/// other services. At some point in the future, the processing will complete,
/// and the `Future` will resolve to a response or error.
///
/// At a high level, the `Service::call` represents an RPC request. The
/// `Service` value can be a server or a client.
///
/// # Server
///
/// An RPC server *implements* the `Service` trait. Requests received by the
/// server over the network are deserialized then passed as an argument to the
/// server value. The returned response is sent back over the network.
///
/// As an example, here is how an HTTP request is processed by a server:
///
/// ```rust,ignore
/// impl Service for HelloWorld {
///     type Req = http::Request;
///     type Resp = http::Response;
///     type Error = http::Error;
///     type Fut = Box<Future<Item = Self::Resp, Error = http::Error>>;
///
///     fn call(&mut self, req: http::Request) -> Self::Fut {
///         // Create the HTTP response
///         let resp = http::Response::ok()
///             .with_body(b"hello world\n");
///
///         // Return the response as an immediate future
///         futures::finished(resp).boxed()
///     }
/// }
/// ```
///
/// # Client
///
/// A client consumes a service by using a `Service` value. The client may
/// issue requests by invoking `call` and passing the request as an argument.
/// It then receives the response by waiting for the returned future.
///
/// As an example, here is how a Redis request would be issued:
///
/// ```rust,ignore
/// let client = redis::Client::new()
///     .connect("127.0.0.1:6379".parse().unwrap())
///     .unwrap();
///
/// let resp = client.call(Cmd::set("foo", "this is the value of foo"));
///
/// // Wait for the future to resolve
/// println!("Redis response: {:?}", await(resp));
/// ```
///
/// # Middleware
///
/// More often than not, all the pieces needed for writing robust, scalable
/// network applications are the same no matter the underlying protocol. By
/// unifying the API for both clients and servers in a protocol agnostic way,
/// it is possible to write middleware that provide these pieces in a
/// reusable way.
///
/// For example, take timeouts as an example:
///
/// ```rust,ignore
/// use tokio::Service;
/// use futures::Future;
/// use std::time::Duration;
///
/// // Not yet implemented, but soon :)
/// use tokio::timer::{Timer, Expired};
///
/// pub struct Timeout<T> {
///     upstream: T,
///     delay: Duration,
///     timer: Timer,
/// }
///
/// impl<T> Timeout<T> {
///     pub fn new(upstream: T, delay: Duration) -> Timeout<T> {
///         Timeout {
///             upstream: upstream,
///             delay: delay,
///             timer: Timer::default(),
///         }
///     }
/// }
///
/// impl<T> Service for Timeout<T>
///     where T: Service,
///           T::Error: From<Expired>,
/// {
///     type Req = T::Req;
///     type Resp = T::Resp;
///     type Error = T::Error;
///     type Fut = Box<Future<Item = Self::Resp, Error = Self::Error>>;
///
///     fn call(&mut self, req: Self::Req) -> Self::Fut {
///         let timeout = self.timer.timeout(self.delay)
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
pub trait Service {

    /// Requests handled by the service.
    type Request;

    /// Responses given by the service.
    type Response;

    /// Errors produced by the service.
    type Error;

    /// The future response value.
    type Future: Future<Item = Self::Response, Error = Self::Error>;

    /// Process the request and return the response asynchronously.
    fn call(&mut self, req: Self::Request) -> AsyncService<Self::Future, Self::Request>;
}

/// The result of an asynchronous attempt to call a service.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AsyncService<T, U> {
    /// The `call` attempt succeeded, so the service has accepted the request.
    Ready(T),

    /// The `call` attempt failed due to the service being at capacity. The request is returned and
    /// the current `Task` will be notified again once the service has capacity.
    NotReady(U),
}

impl<T, U> AsyncService<T, U> {
    /// Returns whether this is `AsyncService::Ready`
    pub fn is_ready(&self) -> bool {
        match *self {
            AsyncService::Ready(_) => true,
            AsyncService::NotReady(_) => false,
        }
    }

    /// Returns whether this is `AsyncService::NotReady`
    pub fn is_not_ready(&self) -> bool {
        !self.is_ready()
    }
}

/// Creates new `Service` values.
pub trait NewService {
    /// Requests handled by the service
    type Request;

    /// Responses given by the service
    type Response;

    /// Errors produced by the service
    type Error;

    /// The `Service` value created by this factory
    type Instance: Service<Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    /// Create and return a new service value.
    fn new_service(&self) -> io::Result<Self::Instance>;
}

impl<F, R> NewService for F
    where F: Fn() -> io::Result<R>,
          R: Service,
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;

    fn new_service(&self) -> io::Result<R> {
        (*self)()
    }
}

impl<S: Service + ?Sized> Service for Box<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&mut self, request: S::Request) -> AsyncService<S::Future, S::Request> {
        (**self).call(request)
    }
}
