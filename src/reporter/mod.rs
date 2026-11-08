use crate::store::Store;
use crate::score::Score;
use crate::triagem::Triagem;
use serde_json::json;

pub struct Reporter;
impl Reporter {
    pub async fn build_payload(store: &Store, score: &Score, triagem: &Triagem, hardware_json: &str) -> String {
        let mut metrics = serde_json::from_str::<serde_json::Value>(hardware_json).unwrap_or(json!({}));
        if let Some(obj) = metrics.as_object_mut() {
            obj.insert("brapi_sla_percent".to_string(), json!(score.get_sla()));
            obj.insert("brapi_ticks_total".to_string(), json!(score.total_ticks.load(std::sync::atomic::Ordering::Relaxed)));
            obj.insert("brapi_failed_ticks".to_string(), json!(score.failed_ticks.load(std::sync::atomic::Ordering::Relaxed)));
            obj.insert("brapi_triagem_errors".to_string(), json!(triagem.error_counts.len()));
            let checkpoint_ok = store.read_state("checkpoint").await.is_some();
            obj.insert("brapi_checkpoint_status".to_string(), json!(if checkpoint_ok { 1 } else { 0 }));
            let last_crash = store.read_state("last_crash").await.unwrap_or_default();
            obj.insert("brapi_last_crash".to_string(), json!(if last_crash == "true" { 1 } else { 0 }));
            obj.insert("log_message".to_string(), json!(format!("Ciclo rodando. SLA: {:.1}%", score.get_sla())));
        }
        metrics.to_string()
    }
}
