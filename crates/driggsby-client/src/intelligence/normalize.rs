#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterpartySource {
    Merchant,
    Description,
}

impl CounterpartySource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Merchant => "merchant",
            Self::Description => "description",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CounterpartyIdentity {
    pub key: String,
    pub label: String,
    pub source: CounterpartySource,
    pub quality_score: f64,
    pub fallback_eligible: bool,
    pub quality_flags: Vec<String>,
}

pub fn counterparty_from_transaction(
    merchant: Option<&str>,
    description: &str,
) -> Option<CounterpartyIdentity> {
    if let Some(merchant_key) = merchant.and_then(normalize_merchant) {
        return Some(CounterpartyIdentity {
            key: merchant_key.clone(),
            label: merchant_key,
            source: CounterpartySource::Merchant,
            quality_score: 1.0,
            fallback_eligible: true,
            quality_flags: vec![
                "counterparty_source:merchant".to_string(),
                "counterparty_quality:strong".to_string(),
            ],
        });
    }

    let fingerprint = description_fingerprint(description)?;
    let token_count = fingerprint.split_whitespace().count();
    let fallback_eligible = token_count >= 2;
    let quality_score = match token_count {
        0 | 1 => 0.55,
        2 => 0.8,
        3 => 0.85,
        _ => 0.9,
    };
    let mut quality_flags = vec!["counterparty_source:description".to_string()];
    if fallback_eligible {
        quality_flags.push("counterparty_quality:description_fallback".to_string());
    } else {
        quality_flags.push("counterparty_quality:weak_description".to_string());
    }

    Some(CounterpartyIdentity {
        key: fingerprint.clone(),
        label: fingerprint,
        source: CounterpartySource::Description,
        quality_score,
        fallback_eligible,
        quality_flags,
    })
}

pub fn normalize_merchant(value: &str) -> Option<String> {
    normalize_text(value)
}

pub fn description_fingerprint(value: &str) -> Option<String> {
    let normalized = normalize_text(value)?;
    let mut stable_tokens: Vec<String> = Vec::new();
    for token in normalized.split_whitespace() {
        if token.is_empty() || is_noise_token(token) || is_numeric_token(token) {
            continue;
        }
        stable_tokens.push(token.to_string());
        if stable_tokens.len() == 4 {
            break;
        }
    }

    if stable_tokens.is_empty() {
        return None;
    }
    Some(stable_tokens.join(" "))
}

fn normalize_text(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut previous_space = false;
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_uppercase());
            previous_space = false;
        } else if !previous_space {
            output.push(' ');
            previous_space = true;
        }
    }

    let normalized = output.trim().to_string();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

fn is_numeric_token(token: &str) -> bool {
    token.chars().all(|character| character.is_ascii_digit())
}

fn is_noise_token(token: &str) -> bool {
    matches!(
        token,
        "POS"
            | "DEBIT"
            | "CARD"
            | "PURCHASE"
            | "ACH"
            | "ONLINE"
            | "PAYMENT"
            | "TRANSFER"
            | "WITHDRAWAL"
            | "CHECK"
            | "ATM"
            | "AUTH"
            | "PENDING"
            | "VISA"
            | "MC"
            | "TRX"
            | "TXN"
    )
}

#[cfg(test)]
mod tests {
    use super::{counterparty_from_transaction, description_fingerprint, normalize_merchant};

    #[test]
    fn merchant_normalization_uppercases_and_collapses_noise() {
        assert_eq!(
            normalize_merchant("  Whole-Foods #123 "),
            Some("WHOLE FOODS 123".to_string())
        );
    }

    #[test]
    fn description_fingerprint_removes_noise_tokens_and_numbers() {
        assert_eq!(
            description_fingerprint("POS DEBIT CARD PURCHASE NETFLIX 1234 MEMBERSHIP"),
            Some("NETFLIX MEMBERSHIP".to_string())
        );
    }

    #[test]
    fn fallback_counterparty_marks_weak_when_only_one_stable_token() {
        let identity = counterparty_from_transaction(None, "ACH PAYMENT NETFLIX 1234");
        assert!(identity.is_some());
        if let Some(counterparty) = identity {
            assert_eq!(counterparty.label, "NETFLIX");
            assert!(!counterparty.fallback_eligible);
        }
    }
}
