mod engine;
mod gate;
mod guard;
mod probe;
mod link;
mod store;
mod recover;
mod restore;
mod learn;
mod score;
mod view;
mod trace;
mod rules;
mod bus;
mod cron;
mod pool;
mod batch;
mod cli;
mod audit;
mod vital;
mod triagem;
mod reporter;
mod control;
mod executor;
mod zelador;
mod predador;

#[tokio::main]
async fn main() {
    let cmd = cli::parse_args();
    if cmd == "start" {
        let engine = engine::Engine::new().await;
        engine.run().await;
    } else if cmd == "version" {
        println!("BRAPI v1.0.0");
    }
}
