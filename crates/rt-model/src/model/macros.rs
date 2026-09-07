//! `merge_field` gövdelerini kısaltan yardımcılar.
//!
//! İkisi de aynı inceliği taşır: okuyucu `None` döndüğünde (wire type uyuşmazlığı,
//! geçersiz UTF-8, bilinmeyen enum) **mevcut değer korunur**. Naif `self.x = ctx.f(f)`
//! yazımı, bozuk bir tekrarın daha önce okunmuş sağlam değeri silmesine yol açardı.

/// Tekil alanı ayarlar; okuma başarısızsa eski değer korunur.
macro_rules! set_if {
    ($target:expr, $read:expr) => {
        if let Some(v) = $read {
            $target = Some(v);
        }
    };
}

/// `repeated` alana ekler; okuma başarısızsa hiçbir şey eklenmez.
macro_rules! push_if {
    ($vec:expr, $read:expr) => {
        if let Some(v) = $read {
            $vec.push(v);
        }
    };
}

pub(crate) use {push_if, set_if};
