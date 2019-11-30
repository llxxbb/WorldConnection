use actix::{Actor, Context, Handler};

use nature_common::Instance;

use crate::actor::MsgForTask;
use crate::controller::channel_parallel;

pub struct ParallelActor;

impl Actor for ParallelActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ParallelActor is started");
    }
}

impl Handler<MsgForTask<Vec<Instance>>> for ParallelActor {
    type Result = ();

    fn handle(&mut self, msg: MsgForTask<Vec<Instance>>, _ctx: &mut Self::Context) -> Self::Result {
        let _ = channel_parallel(msg);
    }
}
