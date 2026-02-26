use serde::Serialize;
use serde_json::Value;

use crate::API_VERSION;
use crate::error::{ClientError, ClientResult};

#[derive(Debug, Clone, Serialize)]
pub struct SuccessEnvelope {
    pub ok: bool,
    pub command: String,
    pub version: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailureEnvelope {
    pub ok: bool,
    pub error: ErrorContract,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorContract {
    pub code: String,
    pub message: String,
    pub recovery_steps: Vec<String>,
}

pub fn success<T>(command: &str, data: T) -> ClientResult<SuccessEnvelope>
where
    T: Serialize,
{
    let json_data = serde_json::to_value(data)
        .map_err(|err| ClientError::internal_serialization(&err.to_string()))?;
    Ok(SuccessEnvelope {
        ok: true,
        command: command.to_string(),
        version: API_VERSION.to_string(),
        data: json_data,
    })
}

pub fn failure_from_error(error: &ClientError) -> FailureEnvelope {
    FailureEnvelope {
        ok: false,
        error: ErrorContract {
            code: error.code.clone(),
            message: error.message.clone(),
            recovery_steps: error.recovery_steps.clone(),
        },
        data: error.data.clone(),
    }
}
