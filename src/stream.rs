use std::marker::PhantomData;
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

pub trait StreamMiddleware<S: StreamService> {
    type WrappedService: StreamService;

    fn wrap(self, service: S) -> Self::WrappedService;

    fn chain<M>(self, middleware: M) -> StreamMiddlewareChain<S, Self, M>
        where M: StreamMiddleware<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareChain {
            inner_middleware: self,
            outer_middleware: middleware,
            _marker: PhantomData,
        }
    }
}

pub struct StreamMiddlewareChain<S, InnerM, OuterM>
    where S: StreamService,
          InnerM: StreamMiddleware<S>,
          OuterM: StreamMiddleware<InnerM::WrappedService>,
{
    inner_middleware: InnerM,
    outer_middleware: OuterM,
    _marker: PhantomData<S>,
}

impl<S, InnerM, OuterM> StreamMiddleware<S> for StreamMiddlewareChain<S, InnerM, OuterM>
    where S: StreamService,
          InnerM: StreamMiddleware<S>,
          OuterM: StreamMiddleware<InnerM::WrappedService>,
{
    type WrappedService = OuterM::WrappedService;

    fn wrap(self, service: S) -> Self::WrappedService {
        service.wrap(self.inner_middleware).wrap(self.outer_middleware)
    }
}
