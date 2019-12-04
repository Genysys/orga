use orga::{Store, Result};
use orga::merkstore::MerkStore;
use orga::abci::{Application, ABCIStateMachine};
use abci2::messages::abci::*;
use byteorder::{ByteOrder, BigEndian};
use failure::bail;

struct CounterApp;

impl CounterApp {
    fn get_count(&self, store: &mut dyn Store) -> Result<u32> {
        match store.get(b"count")? {
            None => Ok(0),
            Some(bytes) => Ok(BigEndian::read_u32(&bytes))
        }
    }

    fn set_count(&self, store: &mut dyn Store, count: u32) -> Result<()> {
        let mut bytes = vec![0; 4];
        BigEndian::write_u32(&mut bytes, count);
        store.put(b"count".to_vec(), bytes)?;
        Ok(())
    }

    fn run(&self, store: &mut dyn Store, tx: &[u8]) -> Result<()> {
        if tx.len() != 4 {
            bail!("Transaction must be exactly 4 bytes");
        }

        let n = BigEndian::read_u32(tx);
        let count = self.get_count(store)?;

        if n != count {
            bail!("Incorrect count");
        }

        self.set_count(store, count + 1)
    }
}

impl Application for CounterApp {
    fn deliver_tx(&self, store: &mut dyn Store, req: RequestDeliverTx) -> Result<ResponseDeliverTx> {
        let bytes = req.get_tx();
        self.run(store, bytes)?;
        Ok(Default::default())
    }

    fn check_tx(&self, store: &mut dyn Store, req: RequestCheckTx) -> Result<ResponseCheckTx> {
        let bytes = req.get_tx();
        self.run(store, bytes)?;
        Ok(Default::default())
    }
}

pub fn main() {
    let mut m = merk::test_utils::TempMerk::new().unwrap();
    let store = MerkStore::new(&mut m);
    ABCIStateMachine::new(CounterApp, store)
        .listen("localhost:26658")
        .unwrap();
}
