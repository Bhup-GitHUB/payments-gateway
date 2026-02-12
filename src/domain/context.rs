use crate::domain::payment::{CreatePaymentRequest, PaymentInstrument};

#[derive(Debug, Clone)]
pub struct PaymentContext {
    pub amount_minor: i64,
    pub currency: String,
    pub merchant_id: String,
    pub method: String,
    pub issuing_bank: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

pub fn build_context(
    req: &CreatePaymentRequest,
    client_ip: Option<String>,
    user_agent: Option<String>,
) -> PaymentContext {
    let issuing_bank = match &req.instrument {
        PaymentInstrument::Card(card) if card.number.len() >= 6 => Some(format!("BIN:{}", &card.number[..6])),
        PaymentInstrument::Upi(upi) => upi.vpa.split('@').nth(1).map(|s| s.to_uppercase()),
        PaymentInstrument::Netbanking(nb) => Some(nb.bank_code.to_uppercase()),
        _ => None,
    };

    PaymentContext {
        amount_minor: req.amount_minor,
        currency: req.currency.clone(),
        merchant_id: req.merchant_id.clone(),
        method: format!("{:?}", req.payment_method),
        issuing_bank,
        client_ip,
        user_agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::payment::{
        CardDetails, CreatePaymentRequest, PaymentInstrument, PaymentMethod, UpiDetails,
    };

    #[test]
    fn derives_bin_for_card() {
        let req = CreatePaymentRequest {
            amount_minor: 100,
            currency: "INR".to_string(),
            payment_method: PaymentMethod::Card,
            merchant_id: "m1".to_string(),
            instrument: PaymentInstrument::Card(CardDetails {
                number: "4111111111111111".to_string(),
                exp_month: 12,
                exp_year: 2030,
                cvv: "123".to_string(),
                name: "A".to_string(),
            }),
        };
        let ctx = build_context(&req, None, None);
        assert_eq!(ctx.issuing_bank.as_deref(), Some("BIN:411111"));
    }

    #[test]
    fn derives_handle_for_upi() {
        let req = CreatePaymentRequest {
            amount_minor: 100,
            currency: "INR".to_string(),
            payment_method: PaymentMethod::Upi,
            merchant_id: "m1".to_string(),
            instrument: PaymentInstrument::Upi(UpiDetails {
                vpa: "user@okhdfcbank".to_string(),
            }),
        };
        let ctx = build_context(&req, None, None);
        assert_eq!(ctx.issuing_bank.as_deref(), Some("OKHDFCBANK"));
    }
}
