use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct IntelligenceFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct NormalizedTransaction {
    pub txn_id: String,
    pub account_key: String,
    pub posted_at: NaiveDate,
    pub amount: f64,
    pub currency: String,
    pub description: String,
    pub merchant: Option<String>,
}

impl NormalizedTransaction {
    pub fn amount_sign_key(&self) -> &'static str {
        if self.amount < 0.0 { "debit" } else { "credit" }
    }

    pub fn abs_amount(&self) -> f64 {
        self.amount.abs()
    }
}
