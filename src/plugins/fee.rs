use crate::call::Call;
use crate::client::Client;
use crate::coins::{Coin, Symbol};
use crate::context::GetContext;
use crate::plugins::Paid;
use crate::query::Query;
use crate::state::State;
use crate::store::Store;
use crate::{Error, Result};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub const MIN_FEE: u64 = 100_000;

pub struct FeePlugin<S, T> {
    inner: T,
    _symbol: PhantomData<S>,
}

impl<S, T> State for FeePlugin<S, T>
where
    S: Symbol,
    T: State,
{
    type Encoding = (T::Encoding,);
    fn create(store: Store, data: Self::Encoding) -> Result<Self> {
        Ok(Self {
            inner: T::create(store, data.0)?,
            _symbol: PhantomData,
        })
    }

    fn flush(self) -> Result<Self::Encoding> {
        Ok((self.inner.flush()?,))
    }
}

impl<S, T> From<FeePlugin<S, T>> for (T::Encoding,)
where
    T: State,
{
    fn from(provider: FeePlugin<S, T>) -> Self {
        (provider.inner.into(),)
    }
}

impl<S, T: Query> Query for FeePlugin<S, T> {
    type Query = T::Query;

    fn query(&self, query: Self::Query) -> Result<()> {
        self.inner.query(query)
    }
}

impl<S: Symbol, T: Call + State> Call for FeePlugin<S, T> {
    type Call = T::Call;

    fn call(&mut self, call: Self::Call) -> Result<()> {
        let paid = self
            .context::<Paid>()
            .ok_or_else(|| Error::Coins("Minimum fee not paid".into()))?;

        let fee_payment: Coin<S> = paid.take(MIN_FEE)?;
        fee_payment.burn();

        self.inner.call(call)
    }
}

impl<S, T: Client<U>, U: Clone> Client<U> for FeePlugin<S, T> {
    type Client = T::Client;

    fn create_client(parent: U) -> Self::Client {
        T::create_client(parent)
    }
}

impl<S, T> Deref for FeePlugin<S, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S, T> DerefMut for FeePlugin<S, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// TODO: Remove dependency on ABCI for this otherwise-pure plugin.
#[cfg(feature = "abci")]
mod abci {
    use super::super::{BeginBlockCtx, EndBlockCtx, InitChainCtx};
    use super::*;
    use crate::abci::{BeginBlock, EndBlock, InitChain};

    impl<S, T> BeginBlock for FeePlugin<S, T>
    where
        S: Symbol,
        T: BeginBlock + State,
    {
        fn begin_block(&mut self, ctx: &BeginBlockCtx) -> Result<()> {
            self.inner.begin_block(ctx)
        }
    }

    impl<S, T> EndBlock for FeePlugin<S, T>
    where
        S: Symbol,
        T: EndBlock + State,
    {
        fn end_block(&mut self, ctx: &EndBlockCtx) -> Result<()> {
            self.inner.end_block(ctx)
        }
    }

    impl<S, T> InitChain for FeePlugin<S, T>
    where
        S: Symbol,
        T: InitChain + State + Call,
    {
        fn init_chain(&mut self, ctx: &InitChainCtx) -> Result<()> {
            self.inner.init_chain(ctx)
        }
    }
}
