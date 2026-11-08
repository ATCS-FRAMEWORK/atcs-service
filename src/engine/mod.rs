use crate::rules::Rules;
use crate::bus::{EventBus, BrapiEvent};
use crate::probe::Probe;
use crate::trace::Tracer;
use crate::gate::Gate;
use crate::vital::Vital;
use crate::store::Store;
use crate::recover::Recover;
use crate::restore::Restore;
use crate::triagem::Triagem;
use crate::learn::Learn;
use crate::score::Score;
use crate::audit::Audit;
use crate::link::Link;
use crate::cron::Cron;
use crate::batch::Batch;
use crate::guard;
use crate::pool::ConnectionPool;
use crate::view;
use crate::reporter::Reporter;
use crate::executor::Executor;
use crate::zelador::Zelador;
use crate::predador::Predador;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::signal;

pub struct Engine {
    pub rules: Rules, pub bus: Arc<EventBus>, pub vital: Vital, pub store: Arc<Store>,
    pub recover: Arc<Recover>, pub restore: Arc<Restore>, pub triagem: Arc<tokio::sync::Mutex<Triagem>>,
    pub learn: Arc<Learn>, pub score: Arc<Score>, pub audit: Arc<Audit>, pub batch: Arc<Batch>,
    pub pool: Arc<tokio::sync::Mutex<ConnectionPool>>, pub executor: Arc<Executor>,
    pub zelador: Arc<Zelador>, pub predador: Arc<Predador>,
}

impl Engine {
    pub async fn new() -> Self {
        let rules = Rules::load();
        let bus = Arc::new(EventBus::new());
        let vital = Vital::new();
        let store = Arc::new(Store::new().await);
        let recover = Arc::new(Recover::new(bus.clone(), store.clone()));
        let restore = Arc::new(Restore::new(store.clone(), recover.clone()));
        let triagem = Arc::new(tokio::sync::Mutex::new(Triagem::new(bus.clone(), store.clone())));
        let learn = Arc::new(Learn::new(store.clone()));
        let score = Arc::new(Score::new());
        let audit = Arc::new(Audit::new());
        let batch = Arc::new(Batch::new(bus.clone(), 5));
        let pool = Arc::new(tokio::sync::Mutex::new(ConnectionPool::new(10)));
        let executor = Arc::new(Executor::new(store.clone()));
        let zelador = Arc::new(Zelador::new(store.clone()));
        let predador = Arc::new(Predador::new(store.clone()));
        Self { rules, bus, vital, store, recover, restore, triagem, learn, score, audit, batch, pool, executor, zelador, predador }
    }

    pub async fn run(&self) {
        if !guard::verify_binary_integrity("") { std::process::exit(1); }
        self.audit.record("STARTUP", &format!("Iniciado. Missao: {}", self.rules.mission_mode));
        self.restore.check_and_restore().await;
        self.bus.publish(BrapiEvent::StateChanged("online".to_string()));
        
        let probe = Probe::new(self.bus.clone(), self.rules.mission_mode == "FULL" || self.rules.mission_mode == "MONITOR");
        probe.start_collection_loop();

        self.zelador.start().await;
        self.predador.start().await;

        if self.rules.mission_mode == "FULL" {
            let tracer = Tracer::new(self.bus.clone(), self.rules.clone());
            tracer.start_listener();
            let gate = Gate::new(self.bus.clone(), self.rules.clone());
            gate.start();
            let link = Link::new(self.bus.clone());
            link.start_network_monitor();
            let cron = Cron::new();
            cron.start_ticks(self.bus.clone());
            view::start_dashboard(self.rules.brapi_port + 1);
        }

        self.vital.start_watchdog();

        let store_ref = self.store.clone(); let score_ref = self.score.clone();
        let triagem_ref = self.triagem.clone(); let bus_ref = self.bus.clone();
        let mut hardware_rx = self.bus.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = hardware_rx.recv().await {
                if let BrapiEvent::MetricCollected(hardware_json) = event {
                    let triagem = triagem_ref.lock().await;
                    let enriched = Reporter::build_payload(&store_ref, &score_ref, &triagem, &hardware_json).await;
                    drop(triagem);
                    bus_ref.publish(BrapiEvent::MetricCollected(enriched));
                }
            }
        });

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let flag_clone = shutdown_flag.clone(); let restore_ref = self.restore.clone();
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
            tokio::select! { _ = sigterm.recv() => {}, _ = sigint.recv() => {}, }
            flag_clone.store(true, Ordering::Relaxed);
            restore_ref.mark_clean_shutdown().await;
        });

        let mut tick_count: u64 = 0;
        let mut ultimo_salvamento = std::time::Instant::now();
        
        loop {
            if shutdown_flag.load(Ordering::Relaxed) { break; }
            self.vital.heartbeat();
            
            let checkpoint_ok = self.store.checkpoint().await;
            self.score.register_tick(checkpoint_ok);
            
            if ultimo_salvamento.elapsed() >= Duration::from_secs(10) {
                let sla = self.score.get_sla();
                self.store.salvar_historico(70.0, 40.0, 50.0, sla).await;
                ultimo_salvamento = std::time::Instant::now();
            }
            
            let decisao = self.learn.analisar_falhas_e_aprender(checkpoint_ok).await;
            
            if tick_count % 5 == 0 {
                let _ = self.batch.push(format!("tick_{}", tick_count));
                let mut p = self.pool.lock().await;
                if p.acquire().is_none() { p.release("conn_healthcheck".to_string()); }
            }
            
            if tick_count % 10 == 0 {
                self.learn.registrar_sucesso("engine_loop").await;
                self.bus.publish(BrapiEvent::SecurityAlert("heartbeat_ok".to_string()));
                self.bus.publish(BrapiEvent::SystemRecovered("health_check".to_string()));
            }
            
            if tick_count % 60 == 0 {
                self.learn.correlacionar_historico().await;
            }

            if decisao == "modo_seguranca" {
                self.store.write_event("LEARN", "Modo de seguranca ativado.").await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            if tick_count % 60 == 0 {
                if !checkpoint_ok {
                    self.store.write_event("WARN", "Checkpoint falhou.").await;
                    let mut t = self.triagem.lock().await; 
                    t.process_error("checkpoint", "falha").await;
                } else {
                    let mut t = self.triagem.lock().await; 
                    t.reset_counters();
                }
            }

            if tick_count % 300 == 0 { 
                self.recover.handle_critical_error("checkpoint_periodic").await; 
            }

            if tick_count % 2 == 0 && self.rules.mission_mode == "FULL" {
                self.executor.process_commands().await;
            }

            tick_count += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
