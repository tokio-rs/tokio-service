use std::io;

use stream::{StreamService, NewStreamService, StreamReduce, NewStreamReduce};

pub trait StreamMiddleware<S: StreamService> {
    type WrappedService: StreamService;

    fn wrap(self, service: S) -> Self::WrappedService;

    fn chain<M>(self, middleware: M) -> StreamMiddlewareChain<Self, M>
        where M: StreamMiddleware<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareChain {
            inner_middleware: self,
            outer_middleware: middleware,
        }
    }

    fn reduce<R>(self, reducer: R) -> StreamMiddlewareReduceChain<Self, R>
        where R: StreamReduce<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareReduceChain {
            middleware: self,
            reducer: reducer,
        }
    }
}

pub struct StreamMiddlewareChain<InnerM, OuterM> {
    inner_middleware: InnerM,
    outer_middleware: OuterM,
}

impl<S, InnerM, OuterM> StreamMiddleware<S> for StreamMiddlewareChain<InnerM, OuterM>
    where S: StreamService,
          InnerM: StreamMiddleware<S>,
          OuterM: StreamMiddleware<InnerM::WrappedService>,
{
    type WrappedService = OuterM::WrappedService;

    fn wrap(self, service: S) -> Self::WrappedService {
        service.wrap(self.inner_middleware).wrap(self.outer_middleware)
    }
}

pub struct StreamMiddlewareReduceChain<M, R> {
    middleware: M,
    reducer: R,
}

impl<S, M, R> StreamReduce<S> for StreamMiddlewareReduceChain<M, R>
    where S: StreamService,
          M: StreamMiddleware<S>,
          R: StreamReduce<M::WrappedService>,
{
    type ReducedService = R::ReducedService;

    fn reduce(self, service: S) -> Self::ReducedService {
        service.wrap(self.middleware).reduce(self.reducer)
    }
}


pub trait NewStreamMiddleware<S: StreamService> {
    type WrappedService: StreamService;
    type Instance: StreamMiddleware<S, WrappedService = Self::WrappedService>;

    fn new_middleware(&self) -> io::Result<Self::Instance>;

    fn wrap<N>(self, new_service: N) -> NewStreamServiceWrapper<Self, N>
        where N: NewStreamService<Instance = S, Request = S::Request, Response = S::Response, Error = S::Error>,
              Self: Sized,
    {
        NewStreamServiceWrapper {
            service: new_service,
            middleware: self,
        }
    }

    fn chain<M>(self, new_middleware: M) -> NewStreamMiddlewareChain<Self, M>
        where M: NewStreamMiddleware<Self::WrappedService>,
              Self: Sized,
    {
        NewStreamMiddlewareChain {
            inner_middleware: self,
            outer_middleware: new_middleware,
        }
    }

    fn reduce<R>(self, new_reducer: R) -> NewStreamMiddlewareReduceChain<Self, R>
        where R: NewStreamReduce<Self::WrappedService>,
              Self: Sized,
    {
        NewStreamMiddlewareReduceChain {
            reducer: new_reducer,
            middleware: self,
        }
    }
}

pub struct NewStreamServiceWrapper<M: NewStreamMiddleware<S::Instance>, S: NewStreamService> {
    service: S,
    middleware: M,
}

impl<M, S, W> NewStreamService for NewStreamServiceWrapper<M, S>
    where S: NewStreamService,
          M: NewStreamMiddleware<S::Instance, WrappedService = W>,
          W: StreamService,
{
    type Request = W::Request;
    type Response = W::Response;
    type Error = W::Error;
    type Instance = W;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.service.new_service()?.wrap(self.middleware.new_middleware()?))
    }
}

pub struct NewStreamMiddlewareChain<InnerM, OuterM> {
    inner_middleware: InnerM,
    outer_middleware: OuterM,
}

impl<S, InnerM, OuterM> NewStreamMiddleware<S> for NewStreamMiddlewareChain<InnerM, OuterM>
    where S: StreamService,
          InnerM: NewStreamMiddleware<S>,
          OuterM: NewStreamMiddleware<InnerM::WrappedService>,
{
    type Instance = StreamMiddlewareChain<InnerM::Instance, OuterM::Instance>;
    type WrappedService = OuterM::WrappedService;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(self.inner_middleware.new_middleware()?.chain(self.outer_middleware.new_middleware()?))
    }
}

pub struct NewStreamMiddlewareReduceChain<M, R> {
    middleware: M,
    reducer: R,
}

impl<S, M, R> NewStreamReduce<S> for NewStreamMiddlewareReduceChain<M, R>
    where S: StreamService,
          M: NewStreamMiddleware<S>,
          R: NewStreamReduce<M::WrappedService>,
{
    type ReducedService = R::ReducedService;
    type Instance = StreamMiddlewareReduceChain<M::Instance, R::Instance>;

    fn new_reducer(&self) -> io::Result<Self::Instance> {
        Ok(self.middleware.new_middleware()?.reduce(self.reducer.new_reducer()?))
    }
}
