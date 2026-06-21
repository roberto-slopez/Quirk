//! QR payload builders for common content types.
//!
//! Each builder returns a UTF-8 string following the de-facto standards for
//! QR-encoded data (wifi, vCard, mailto, tel:, geo:, vEvent, etc.). Plain text
//! and URLs are passed through verbatim.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrKind {
    Text,
    Url,
    Wifi,
    VCard,
    Email,
    Phone,
    Sms,
    Geo,
    Event,
    Crypto,
}

impl QrKind {
    pub const ALL: &'static [QrKind] = &[
        QrKind::Text,
        QrKind::Url,
        QrKind::Wifi,
        QrKind::VCard,
        QrKind::Email,
        QrKind::Phone,
        QrKind::Sms,
        QrKind::Geo,
        QrKind::Event,
        QrKind::Crypto,
    ];

    pub fn label(self) -> &'static str {
        match self {
            QrKind::Text => "Text",
            QrKind::Url => "URL",
            QrKind::Wifi => "Wi-Fi",
            QrKind::VCard => "Contact (vCard)",
            QrKind::Email => "Email",
            QrKind::Phone => "Phone",
            QrKind::Sms => "SMS",
            QrKind::Geo => "Location",
            QrKind::Event => "Calendar event",
            QrKind::Crypto => "Crypto payment",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiParams {
    pub ssid: String,
    pub password: String,
    pub security: WifiSecurity,
    pub hidden: bool,
}

impl Default for WifiParams {
    fn default() -> Self {
        Self {
            ssid: String::new(),
            password: String::new(),
            security: WifiSecurity::Wpa,
            hidden: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiSecurity {
    Wpa,
    Wep,
    /// "nopass" — open network
    None,
}

impl WifiSecurity {
    pub const ALL: &'static [WifiSecurity] =
        &[WifiSecurity::Wpa, WifiSecurity::Wep, WifiSecurity::None];

    pub fn label(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA / WPA2",
            WifiSecurity::Wep => "WEP",
            WifiSecurity::None => "Open (no password)",
        }
    }

    fn token(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA",
            WifiSecurity::Wep => "WEP",
            WifiSecurity::None => "nopass",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VCardParams {
    pub full_name: String,
    pub org: String,
    pub title: String,
    pub phone: String,
    pub email: String,
    pub url: String,
    pub address: String,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct EmailParams {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct SmsParams {
    pub phone: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct GeoParams {
    pub latitude: String,
    pub longitude: String,
    pub altitude: Option<String>,
    pub query: String, // optional label for the location
}

#[derive(Debug, Clone, Default)]
pub struct EventParams {
    pub summary: String,
    pub location: String,
    pub description: String,
    pub start: String, // ISO 8601 local datetime, e.g. 2026-01-31T18:00
    pub end: String,
    pub all_day: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CryptoKind {
    #[default]
    Bitcoin,
    Ethereum,
}

impl CryptoKind {
    pub const ALL: &'static [CryptoKind] = &[CryptoKind::Bitcoin, CryptoKind::Ethereum];

    pub fn label(self) -> &'static str {
        match self {
            CryptoKind::Bitcoin => "Bitcoin",
            CryptoKind::Ethereum => "Ethereum",
        }
    }

    fn scheme(self) -> &'static str {
        match self {
            CryptoKind::Bitcoin => "bitcoin",
            CryptoKind::Ethereum => "ethereum",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CryptoParams {
    pub kind: CryptoKind,
    pub address: String,
    pub amount: String,
    pub label: String,
    pub message: String,
}

/// Escape characters that have special meaning in the wifi/vCard URI syntax.
/// We backslash-escape `\`, `;`, `,`, `"` and `:`.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ';' | ',' | '"' | ':' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

pub fn build(kind: QrKind, parts: &QrParts) -> Option<String> {
    let s = match kind {
        QrKind::Text => parts.text.trim().to_string(),
        QrKind::Url => {
            let t = parts.text.trim();
            if t.is_empty() {
                return None;
            }
            if !t.contains("://") && !t.starts_with("mailto:") && !t.starts_with("tel:") {
                format!("https://{}", t)
            } else {
                t.to_string()
            }
        }
        QrKind::Wifi => build_wifi(&parts.wifi),
        QrKind::VCard => build_vcard(&parts.vcard),
        QrKind::Email => build_email(&parts.email),
        QrKind::Phone => {
            let p = parts.phone.phone.trim();
            if p.is_empty() {
                return None;
            }
            format!("tel:{}", p)
        }
        QrKind::Sms => build_sms(&parts.sms),
        QrKind::Geo => build_geo(&parts.geo),
        QrKind::Event => build_event(&parts.event),
        QrKind::Crypto => build_crypto(&parts.crypto),
    };
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Bag of typed parameters keyed by [`QrKind`]. Only the relevant field is
/// read when building, the others are kept around so the UI state survives
/// switching tabs.
#[derive(Debug, Clone, Default)]
pub struct QrParts {
    pub text: String,
    pub wifi: WifiParams,
    pub vcard: VCardParams,
    pub email: EmailParams,
    pub phone: SmsParams, // phone reuses SmsParams (phone + optional msg unused)
    pub sms: SmsParams,
    pub geo: GeoParams,
    pub event: EventParams,
    pub crypto: CryptoParams,
}

fn build_wifi(p: &WifiParams) -> String {
    if p.ssid.trim().is_empty() {
        return String::new();
    }
    let sec = p.security.token();
    let hidden = if p.hidden { "true" } else { "false" };
    format!(
        "WIFI:T:{};S:{};P:{};H:{};;",
        sec,
        esc(&p.ssid),
        esc(&p.password),
        hidden,
    )
}

fn build_vcard(p: &VCardParams) -> String {
    if p.full_name.trim().is_empty() {
        return String::new();
    }
    let mut out = String::from("BEGIN:VCARD\r\nVERSION:3.0\r\n");
    out.push_str(&format!("FN:{}\r\n", esc(&p.full_name)));
    let parts: Vec<&str> = p.full_name.split_whitespace().collect();
    if let (Some(last), Some(first)) = (parts.last(), parts.first()) {
        out.push_str(&format!("N:{};{};;;\r\n", esc(last), esc(first)));
    }
    if !p.org.trim().is_empty() {
        out.push_str(&format!("ORG:{}\r\n", esc(&p.org)));
    }
    if !p.title.trim().is_empty() {
        out.push_str(&format!("TITLE:{}\r\n", esc(&p.title)));
    }
    if !p.phone.trim().is_empty() {
        out.push_str(&format!("TEL:{}\r\n", esc(&p.phone)));
    }
    if !p.email.trim().is_empty() {
        out.push_str(&format!("EMAIL:{}\r\n", esc(&p.email)));
    }
    if !p.url.trim().is_empty() {
        out.push_str(&format!("URL:{}\r\n", esc(&p.url)));
    }
    if !p.address.trim().is_empty() {
        out.push_str(&format!("ADR:;;{};;;;\r\n", esc(&p.address)));
    }
    if !p.note.trim().is_empty() {
        out.push_str(&format!("NOTE:{}\r\n", esc(&p.note)));
    }
    out.push_str("END:VCARD\r\n");
    out
}

fn build_email(p: &EmailParams) -> String {
    if p.to.trim().is_empty() {
        return String::new();
    }
    let mut url = format!("mailto:{}", url_encode(&p.to));
    let mut qs = Vec::new();
    if !p.subject.trim().is_empty() {
        qs.push(format!("subject={}", url_encode(&p.subject)));
    }
    if !p.body.trim().is_empty() {
        qs.push(format!("body={}", url_encode(&p.body)));
    }
    if !qs.is_empty() {
        url.push('?');
        url.push_str(&qs.join("&"));
    }
    url
}

fn build_sms(p: &SmsParams) -> String {
    if p.phone.trim().is_empty() {
        return String::new();
    }
    if p.message.trim().is_empty() {
        format!("smsto:{}", p.phone.trim())
    } else {
        format!("smsto:{}:{}", p.phone.trim(), url_encode(&p.message))
    }
}

fn build_geo(p: &GeoParams) -> String {
    if p.latitude.trim().is_empty() || p.longitude.trim().is_empty() {
        return String::new();
    }
    let mut url = format!("geo:{},{}", p.latitude.trim(), p.longitude.trim());
    if let Some(alt) = p.altitude.as_ref().filter(|s| !s.trim().is_empty()) {
        url.push(',');
        url.push_str(alt.trim());
    }
    if !p.query.trim().is_empty() {
        url.push('?');
        url.push_str(&url_encode(&p.query));
    }
    url
}

fn build_event(p: &EventParams) -> String {
    if p.summary.trim().is_empty() || p.start.trim().is_empty() {
        return String::new();
    }
    let end = if p.end.trim().is_empty() {
        p.start.clone()
    } else {
        p.end.clone()
    };
    let (start, end) = if p.all_day {
        // vCalendar expects DATE values for all-day.
        (strip_time(&p.start), strip_time(&end))
    } else {
        (p.start.clone(), end)
    };
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\n");
    out.push_str(&format!("SUMMARY:{}\r\n", esc(&p.summary)));
    if !p.location.trim().is_empty() {
        out.push_str(&format!("LOCATION:{}\r\n", esc(&p.location)));
    }
    if !p.description.trim().is_empty() {
        out.push_str(&format!("DESCRIPTION:{}\r\n", esc(&p.description)));
    }
    if p.all_day {
        out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", start));
        out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end));
    } else {
        out.push_str(&format!("DTSTART:{}\r\n", start));
        out.push_str(&format!("DTEND:{}\r\n", end));
    }
    out.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
    out
}

fn build_crypto(p: &CryptoParams) -> String {
    if p.address.trim().is_empty() {
        return String::new();
    }
    let mut url = format!("{}:{}", p.kind.scheme(), p.address.trim());
    let mut qs = Vec::new();
    if !p.amount.trim().is_empty() {
        qs.push(format!("amount={}", url_encode(&p.amount)));
    }
    if !p.label.trim().is_empty() {
        qs.push(format!("label={}", url_encode(&p.label)));
    }
    if !p.message.trim().is_empty() {
        qs.push(format!("message={}", url_encode(&p.message)));
    }
    if !qs.is_empty() {
        url.push('?');
        url.push_str(&qs.join("&"));
    }
    url
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn strip_time(s: &str) -> String {
    // Accept "YYYY-MM-DDTHH:MM" or "YYYY-MM-DD HH:MM" and keep just the date.
    s.split(['T', ' ', 't']).next().unwrap_or(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_prepends_https_when_missing() {
        let p = QrParts {
            text: "example.com/path".into(),
            ..Default::default()
        };
        assert_eq!(build(QrKind::Url, &p).unwrap(), "https://example.com/path");
    }

    #[test]
    fn url_passes_through_with_scheme() {
        let p = QrParts {
            text: "ftp://example.com".into(),
            ..Default::default()
        };
        assert_eq!(build(QrKind::Url, &p).unwrap(), "ftp://example.com");
    }

    #[test]
    fn wifi_encodes_correctly() {
        let p = QrParts {
            wifi: WifiParams {
                ssid: "My Net".into(),
                password: "p@ss;word".into(),
                security: WifiSecurity::Wpa,
                hidden: false,
            },
            ..Default::default()
        };
        assert_eq!(
            build(QrKind::Wifi, &p).unwrap(),
            r#"WIFI:T:WPA;S:My Net;P:p@ss\;word;H:false;;"#
        );
    }

    #[test]
    fn vcard_contains_minimum() {
        let p = QrParts {
            vcard: VCardParams {
                full_name: "Ada Lovelace".into(),
                email: "ada@example.com".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = build(QrKind::VCard, &p).unwrap();
        assert!(out.starts_with("BEGIN:VCARD\r\n"));
        assert!(out.contains("FN:Ada Lovelace"));
        assert!(out.contains("EMAIL:ada@example.com"));
        assert!(out.ends_with("END:VCARD\r\n"));
    }

    #[test]
    fn crypto_btc_uri() {
        let p = QrParts {
            crypto: CryptoParams {
                kind: CryptoKind::Bitcoin,
                address: "1BoatSLRHtKNngkdXEeobR76b53LETtpyT".into(),
                amount: "0.001".into(),
                label: "Donation".into(),
                message: String::new(),
            },
            ..Default::default()
        };
        let out = build(QrKind::Crypto, &p).unwrap();
        assert!(out.starts_with("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?"));
        assert!(out.contains("amount=0.001"));
        assert!(out.contains("label=Donation"));
    }

    #[test]
    fn empty_inputs_return_none() {
        let p = QrParts::default();
        assert!(build(QrKind::Text, &p).is_none());
        assert!(build(QrKind::Wifi, &p).is_none());
        assert!(build(QrKind::VCard, &p).is_none());
        assert!(build(QrKind::Phone, &p).is_none());
    }

    #[test]
    fn event_all_day_strips_time() {
        let p = QrParts {
            event: EventParams {
                summary: "Birthday".into(),
                start: "2026-12-31T18:00".into(),
                end: "2026-12-31T22:00".into(),
                all_day: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = build(QrKind::Event, &p).unwrap();
        assert!(out.contains("DTSTART;VALUE=DATE:2026-12-31"));
        assert!(out.contains("DTEND;VALUE=DATE:2026-12-31"));
    }
}
