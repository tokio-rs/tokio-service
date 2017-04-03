mod middleware;
mod reduce;

pub use self::middleware::*;
pub use self::reduce::*;

use std::io;
use std::rc::Rc;
use std::sync::Arc;

use futures::Stream;

pub trait StreamService {
    type Request;
    type Response;
    type Error;
    type Stream: Stream<Item = Self::Response, Error = Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Stream;

    fn wrap<M>(self, middleware: M) -> M::WrappedService
        where M: StreamMiddleware<Self>,
              Self: Sized,
    {
        middleware.wrap(self)
    }

    fn reduce<R>(self, reducer: R) -> R::ReducedService
        where R: StreamReduce<Self>,
              Self: Sized,
    {
        reducer.reduce(self)
    }
}

impl<S: StreamService + ?Sized> StreamService for Box<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}

impl<S: StreamService + ?Sized> StreamService for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}

impl<S: StreamService + ?Sized> StreamService for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}

pub trait NewStreamService {
    type Request;
    type Response;
    type Error;
    type Instance: StreamService<Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    fn new_service(&self) -> io::Result<Self::Instance>;

    fn wrap<M>(self, new_middleware: M) -> NewStreamServiceWrapper<M, Self>
        where M: NewStreamMiddleware<Self::Instance>,
              Self: Sized,
    {
        new_middleware.wrap(self)
    }

    fn reduce<R>(self, new_reducer: R) -> NewStreamServiceReducer<R, Self>
        where R: NewStreamReduce<Self::Instance>,
              Self: Sized,
    {
        new_reducer.reduce(self)
    }
}

impl<F, R> NewStreamService for F
    where F: Fn() -> io::Result<R>,
          R: StreamService,
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;

    fn new_service(&self) -> io::Result<R> {
        (*self)()
    }
}

impl<S: NewStreamService + ?Sized> NewStreamService for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;

    fn new_service(&self) -> io::Result<S::Instance> {
        (**self).new_service()
    }
}

impl<S: NewStreamService + ?Sized> NewStreamService for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;

    fn new_service(&self) -> io::Result<S::Instance> {
        (**self).new_service()
    }
}
