use std::io;
use std::marker::PhantomData;

use {Middleware, NewMiddleware, Service, NewService};
use stream::{StreamService, NewStreamService};

pub trait StreamReduce<S: StreamService> {
    type ReducedService: Service;

    fn reduce(self, service: S) -> Self::ReducedService;

    fn chain<M>(self, middleware: M) -> StreamReduceMiddlewareChain<S, Self, M>
        where M: Middleware<Self::ReducedService>,
              Self: Sized,
    {
        StreamReduceMiddlewareChain {
            reducer: self,
            middleware: middleware,
            _marker: PhantomData,
        }
    }
}

pub struct StreamReduceMiddlewareChain<S, R, M>
    where S: StreamService,
          R: StreamReduce<S>,
          M: Middleware<R::ReducedService>,
{
    reducer: R,
    middleware: M,
    _marker: PhantomData<S>,
}

impl<S, R, M> StreamReduce<S> for StreamReduceMiddlewareChain<S, R, M>
    where S: StreamService,
          R: StreamReduce<S>,
          M: Middleware<R::ReducedService>,
{
    type ReducedService = M::WrappedService;

    fn reduce(self, service: S) -> Self::ReducedService {
        service.reduce(self.reducer).wrap(self.middleware)
    }
}

pub trait NewStreamReduce<S: StreamService> {
    type ReducedService: Service;
    type Instance: StreamReduce<S, ReducedService = Self::ReducedService>;

    fn new_reducer(&self) -> io::Result<Self::Instance>;

    fn reduce<N>(self, new_service: N) -> NewStreamServiceReducer<Self, N>
        where N: NewStreamService<Instance = S, Request = S::Request, Response = S::Response, Error = S::Error>,
              Self: Sized,
    {
        NewStreamServiceReducer {
            service: new_service,
            reducer: self,
        }
    }

    fn chain<M>(self, new_middleware: M) -> NewStreamReduceMiddlewareChain<S, Self, M>
        where M: NewMiddleware<Self::ReducedService>,
              Self: Sized,
    {
        NewStreamReduceMiddlewareChain {
            reducer: self,
            middleware: new_middleware,
            _marker: PhantomData,
        }
    }
}

pub struct NewStreamServiceReducer<R: NewStreamReduce<S::Instance>, S: NewStreamService> {
    service: S,
    reducer: R,
}

impl<R, S, W> NewService for NewStreamServiceReducer<R, S>
    where S: NewStreamService,
          R: NewStreamReduce<S::Instance, ReducedService = W>,
          W: Service,
{
    type Request = W::Request;
    type Response = W::Response;
    type Error = W::Error;
    type Instance = W;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.service.new_service()?.reduce(self.reducer.new_reducer()?))
    }
}

pub struct NewStreamReduceMiddlewareChain<S, R, M>
where
    S: StreamService,
    R: NewStreamReduce<S>,
    M: NewMiddleware<R::ReducedService>,
{
    reducer: R,
    middleware: M,
    _marker: PhantomData<S>,
}

impl<S, R, M> NewStreamReduce<S> for NewStreamReduceMiddlewareChain<S, R, M>
where
    S: StreamService,
    R: NewStreamReduce<S>,
    M: NewMiddleware<R::ReducedService>,
{
    type ReducedService = M::WrappedService;
    type Instance = StreamReduceMiddlewareChain<S, R::Instance, M::Instance>;

    fn new_reducer(&self) -> io::Result<Self::Instance> {
        Ok(self.reducer.new_reducer()?.chain(self.middleware.new_middleware()?))
    }
}
