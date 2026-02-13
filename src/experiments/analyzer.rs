use crate::domain::experiment::ExperimentResultRow;

#[derive(Debug, Clone, serde::Serialize)]
pub struct WinnerAnalysis {
    pub control_success_rate: f64,
    pub treatment_success_rate: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub winner: Option<String>,
    pub recommendation: String,
}

pub fn analyze(results: &[ExperimentResultRow], min_samples: i64) -> WinnerAnalysis {
    let (c_total, c_success) = aggregate_variant(results, "control");
    let (t_total, t_success) = aggregate_variant(results, "treatment");

    if c_total < min_samples || t_total < min_samples || c_total == 0 || t_total == 0 {
        return WinnerAnalysis {
            control_success_rate: ratio(c_success, c_total),
            treatment_success_rate: ratio(t_success, t_total),
            z_score: 0.0,
            p_value: 1.0,
            is_significant: false,
            winner: None,
            recommendation: "insufficient sample size".to_string(),
        };
    }

    let p1 = ratio(c_success, c_total);
    let p2 = ratio(t_success, t_total);
    let pooled = ratio(c_success + t_success, c_total + t_total);
    let se = (pooled * (1.0 - pooled) * ((1.0 / c_total as f64) + (1.0 / t_total as f64))).sqrt();

    if se == 0.0 {
        return WinnerAnalysis {
            control_success_rate: p1,
            treatment_success_rate: p2,
            z_score: 0.0,
            p_value: 1.0,
            is_significant: false,
            winner: None,
            recommendation: "unable to compute significance".to_string(),
        };
    }

    let z = (p2 - p1) / se;
    let p = 2.0 * (1.0 - normal_cdf(z.abs()));
    let significant = p < 0.05;
    let winner = if significant {
        if p2 > p1 {
            Some("treatment".to_string())
        } else {
            Some("control".to_string())
        }
    } else {
        None
    };

    WinnerAnalysis {
        control_success_rate: p1,
        treatment_success_rate: p2,
        z_score: z,
        p_value: p,
        is_significant: significant,
        winner: winner.clone(),
        recommendation: match winner {
            Some(ref w) if w == "treatment" => "promote treatment".to_string(),
            Some(_) => "keep control".to_string(),
            None => "continue experiment".to_string(),
        },
    }
}

fn aggregate_variant(results: &[ExperimentResultRow], variant: &str) -> (i64, i64) {
    let mut total = 0_i64;
    let mut success = 0_i64;
    for row in results {
        if row.variant == variant {
            total += row.total_requests;
            success += row.successful_requests;
        }
    }
    (total, success)
}

fn ratio(a: i64, b: i64) -> f64 {
    if b <= 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

fn normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989423 * (-x * x / 2.0).exp();
    let prob = 1.0
        - d * t
            * (0.3193815
                + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));
    if x >= 0.0 { prob } else { 1.0 - prob }
}
