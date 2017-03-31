use std::marker::PhantomData;

use {Middleware, Service};
use stream::StreamService;

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

    fn reduce<R>(self, reducer: R) -> StreamMiddlewareReduceChain<S, Self, R>
        where R: StreamReduce<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareReduceChain {
            middleware: self,
            reducer: reducer,
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

pub struct StreamMiddlewareReduceChain<S, M, R>
    where S: StreamService,
          M: StreamMiddleware<S>,
          R: StreamReduce<M::WrappedService>,
{
    middleware: M,
    reducer: R,
    _marker: PhantomData<S>,
}

impl<S, M, R> StreamReduce<S> for StreamMiddlewareReduceChain<S, M, R>
    where S: StreamService,
          M: StreamMiddleware<S>,
          R: StreamReduce<M::WrappedService>,
{
    type ReducedService = R::ReducedService;

    fn reduce(self, service: S) -> Self::ReducedService {
        service.wrap(self.middleware).reduce(self.reducer)
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

pub trait NewStreamMiddleware<S: StreamService> {
    type Instance: StreamMiddleware<S>;
    fn new_middleware(&self) -> Self::Instance;
}

pub trait NewStreamReduce<S: StreamService> {
    type Instance: StreamReduce<S>;
    fn new_reducer(&self) -> Self::Instance;
}
