//! utils/phone.rs — normalisasi nomor HP Indonesia ke format `62xxxxxxxxxx`
//! (dipakai sebagai identitas login DAN untuk membentuk `chatId` WAHA).

/// Terima `08xx`, `+62xx`, `62xx`, atau `8xx` (spasi/tanda hubung/kurung
/// diabaikan), kembalikan bentuk `62xxxxxxxxxx`. `None` bila panjangnya tak
/// masuk akal untuk nomor Indonesia (mencegah salah ketik yang lolos begitu
/// saja jadi "akun" baru).
pub fn normalize(input: &str) -> Option<String> {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }

    let with_country = if let Some(rest) = digits.strip_prefix('0') {
        format!("62{rest}")
    } else if digits.starts_with("62") {
        digits
    } else if digits.starts_with('8') {
        format!("62{digits}")
    } else {
        return None;
    };

    // 62 + 8xx... : total realistis 10–15 digit.
    if with_country.len() < 10 || with_country.len() > 15 || !with_country.starts_with("628") {
        return None;
    }

    Some(with_country)
}

/// Bentuk `chatId` WAHA dari nomor yang SUDAH dinormalisasi.
pub fn to_waha_chat_id(normalized: &str) -> String {
    format!("{normalized}@c.us")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nol_diawal_diganti_62() {
        assert_eq!(normalize("081234567890").as_deref(), Some("6281234567890"));
    }

    #[test]
    fn sudah_62_dipertahankan() {
        assert_eq!(normalize("+62 812-3456-7890").as_deref(), Some("6281234567890"));
    }

    #[test]
    fn tanpa_awalan_ditambah_62() {
        assert_eq!(normalize("81234567890").as_deref(), Some("6281234567890"));
    }

    #[test]
    fn bukan_nomor_indonesia_ditolak() {
        assert_eq!(normalize("1234"), None);
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("021-5551234"), None); // nomor rumah, bukan 08xx
    }

    #[test]
    fn chat_id_terbentuk() {
        assert_eq!(to_waha_chat_id("6281234567890"), "6281234567890@c.us");
    }
}
