//! Helpers for showing errors returned by a Wally registry.

use serde::Deserialize;

#[derive(Deserialize)]
struct ErrorResponse {
    message: String,
}

/// Turns the body of a failed registry response into something readable.
///
/// The registry returns errors as `{"message": "..."}`. If the body has that
/// shape, only the message is returned; otherwise the body is returned as-is.
pub fn registry_error_message(body: String) -> String {
    match serde_json::from_str::<ErrorResponse>(&body) {
        Ok(response) => response.message,
        Err(_) => body,
    }
}

#[cfg(test)]
mod tests {
    use super::registry_error_message;

    #[test]
    fn extracts_message_from_json() {
        assert_eq!(
            registry_error_message(r#"{"message":"413 Payload Too Large"}"#.to_owned()),
            "413 Payload Too Large"
        );
    }

    #[test]
    fn keeps_other_bodies_unchanged() {
        assert_eq!(registry_error_message("oops".to_owned()), "oops");
        assert_eq!(
            registry_error_message(r#"{"error":"x"}"#.to_owned()),
            r#"{"error":"x"}"#
        );
    }
}
