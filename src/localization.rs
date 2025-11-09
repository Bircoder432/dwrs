use locale_config::Locale;
use once_cell::sync::Lazy;

rust_i18n::i18n!("i18n");

pub static INIT_LOCALE: Lazy<()> = Lazy::new(|| {
    let system_locale = Locale::user_default().to_string();
    let short_locale = system_locale.split('_').next().unwrap_or("en");
    rust_i18n::set_locale(short_locale);
});

pub fn init_locale() {
    Lazy::force(&INIT_LOCALE);
}
