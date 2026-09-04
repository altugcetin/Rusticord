#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Locale {
    #[default]
    Turkish,
    English,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    ApplicationName,
    AppearanceDark,
    AppearanceLight,
    ToggleAppearance,
    GuildRailTitle,
    ChannelSidebarTitle,
    ChatHeaderPlaceholder,
    MemberListTitle,
    EmptyGuildsTitle,
    EmptyGuildsBody,
    EmptyChannelsTitle,
    EmptyChannelsBody,
    EmptyChatTitle,
    EmptyChatBody,
    EmptyMembersTitle,
    EmptyMembersBody,
    RiskNoticeTitle,
    RiskNoticeBody,
    RiskNoticeAccept,
    LoginTitle,
    LoginIdentifier,
    LoginPassword,
    LoginSubmit,
    LoginBusy,
    LoginErrorGeneric,
    LoginInvalidAuth,
    LoginNetworkError,
    MfaTitle,
    MfaCode,
    MfaSubmit,
    CaptchaTitle,
    CaptchaBody,
    CaptchaKey,
    CaptchaSubmit,
}

impl MessageKey {
    #[cfg(test)]
    const ALL: [Self; 34] = [
        Self::ApplicationName,
        Self::AppearanceDark,
        Self::AppearanceLight,
        Self::ToggleAppearance,
        Self::GuildRailTitle,
        Self::ChannelSidebarTitle,
        Self::ChatHeaderPlaceholder,
        Self::MemberListTitle,
        Self::EmptyGuildsTitle,
        Self::EmptyGuildsBody,
        Self::EmptyChannelsTitle,
        Self::EmptyChannelsBody,
        Self::EmptyChatTitle,
        Self::EmptyChatBody,
        Self::EmptyMembersTitle,
        Self::EmptyMembersBody,
        Self::RiskNoticeTitle,
        Self::RiskNoticeBody,
        Self::RiskNoticeAccept,
        Self::LoginTitle,
        Self::LoginIdentifier,
        Self::LoginPassword,
        Self::LoginSubmit,
        Self::LoginBusy,
        Self::LoginErrorGeneric,
        Self::LoginInvalidAuth,
        Self::LoginNetworkError,
        Self::MfaTitle,
        Self::MfaCode,
        Self::MfaSubmit,
        Self::CaptchaTitle,
        Self::CaptchaBody,
        Self::CaptchaKey,
        Self::CaptchaSubmit,
    ];
}

pub fn translate(locale: Locale, key: MessageKey) -> &'static str {
    match locale {
        Locale::Turkish => turkish(key),
        Locale::English => english(key),
    }
}

fn turkish(key: MessageKey) -> &'static str {
    match key {
        MessageKey::ApplicationName => "Rusticord",
        MessageKey::AppearanceDark => "Koyu tema",
        MessageKey::AppearanceLight => "Açık tema",
        MessageKey::ToggleAppearance => "Temayı değiştir",
        MessageKey::GuildRailTitle => "Sunucular",
        MessageKey::ChannelSidebarTitle => "Kanallar",
        MessageKey::ChatHeaderPlaceholder => "Sohbet",
        MessageKey::MemberListTitle => "Üyeler",
        MessageKey::EmptyGuildsTitle => "Sunucu yok",
        MessageKey::EmptyGuildsBody => "Bağlı bir sunucu olduğunda burada durur.",
        MessageKey::EmptyChannelsTitle => "Kanal yok",
        MessageKey::EmptyChannelsBody => "Bir sunucu seçildiğinde kanallar burada durur.",
        MessageKey::EmptyChatTitle => "Henüz ileti yok",
        MessageKey::EmptyChatBody => "Bu kanal sessiz. Bir ileti yazıldığında burada görünür.",
        MessageKey::EmptyMembersTitle => "Üye yok",
        MessageKey::EmptyMembersBody => "Üyeler bir kanal açıldığında burada durur.",
        MessageKey::RiskNoticeTitle => "Hesap riski",
        MessageKey::RiskNoticeBody => {
            "Üçüncü taraf istemciler Discord hizmet şartlarına aykırıdır. Devam etmek hesap riskini kabul etmektir."
        }
        MessageKey::RiskNoticeAccept => "Riski kabul ediyorum",
        MessageKey::LoginTitle => "Giriş",
        MessageKey::LoginIdentifier => "E-posta veya telefon",
        MessageKey::LoginPassword => "Parola",
        MessageKey::LoginSubmit => "Giriş yap",
        MessageKey::LoginBusy => "Giriş yapılıyor",
        MessageKey::LoginErrorGeneric => "Giriş başarısız oldu.",
        MessageKey::LoginInvalidAuth => "Kimlik bilgileri geçersiz.",
        MessageKey::LoginNetworkError => "Ağa ulaşılamadı.",
        MessageKey::MfaTitle => "İki adımlı doğrulama",
        MessageKey::MfaCode => "Doğrulama kodu",
        MessageKey::MfaSubmit => "Doğrula",
        MessageKey::CaptchaTitle => "Doğrulama gerekli",
        MessageKey::CaptchaBody => "Discord bir captcha istedi. Çözüm anahtarını yapıştırın.",
        MessageKey::CaptchaKey => "Captcha anahtarı",
        MessageKey::CaptchaSubmit => "Yeniden dene",
    }
}

fn english(key: MessageKey) -> &'static str {
    match key {
        MessageKey::ApplicationName => "Rusticord",
        MessageKey::AppearanceDark => "Dark theme",
        MessageKey::AppearanceLight => "Light theme",
        MessageKey::ToggleAppearance => "Toggle theme",
        MessageKey::GuildRailTitle => "Servers",
        MessageKey::ChannelSidebarTitle => "Channels",
        MessageKey::ChatHeaderPlaceholder => "Chat",
        MessageKey::MemberListTitle => "Members",
        MessageKey::EmptyGuildsTitle => "No servers",
        MessageKey::EmptyGuildsBody => "Servers you belong to will sit here.",
        MessageKey::EmptyChannelsTitle => "No channels",
        MessageKey::EmptyChannelsBody => "Channels appear here when a server is selected.",
        MessageKey::EmptyChatTitle => "No messages yet",
        MessageKey::EmptyChatBody => {
            "This channel is quiet. Messages will show here when someone writes."
        }
        MessageKey::EmptyMembersTitle => "No members",
        MessageKey::EmptyMembersBody => "Members appear here when a channel is open.",
        MessageKey::RiskNoticeTitle => "Account risk",
        MessageKey::RiskNoticeBody => {
            "Third party clients are against Discord terms of service. Continuing accepts the account risk."
        }
        MessageKey::RiskNoticeAccept => "I accept the risk",
        MessageKey::LoginTitle => "Sign in",
        MessageKey::LoginIdentifier => "Email or phone",
        MessageKey::LoginPassword => "Password",
        MessageKey::LoginSubmit => "Sign in",
        MessageKey::LoginBusy => "Signing in",
        MessageKey::LoginErrorGeneric => "Sign in failed.",
        MessageKey::LoginInvalidAuth => "Those credentials are not valid.",
        MessageKey::LoginNetworkError => "The network could not be reached.",
        MessageKey::MfaTitle => "Two-step verification",
        MessageKey::MfaCode => "Verification code",
        MessageKey::MfaSubmit => "Verify",
        MessageKey::CaptchaTitle => "Verification required",
        MessageKey::CaptchaBody => "Discord asked for a captcha. Paste the solution key.",
        MessageKey::CaptchaKey => "Captcha key",
        MessageKey::CaptchaSubmit => "Try again",
    }
}

#[cfg(test)]
mod tests {
    use super::{Locale, MessageKey, translate};

    #[test]
    fn turkish_is_the_default_locale() {
        assert_eq!(Locale::default(), Locale::Turkish);
    }

    #[test]
    fn english_differs_on_theme_toggle() {
        let turkish = translate(Locale::Turkish, MessageKey::ToggleAppearance);
        let english = translate(Locale::English, MessageKey::ToggleAppearance);
        assert_ne!(turkish, english);
    }

    #[test]
    fn no_em_dash_in_any_locale() {
        for key in MessageKey::ALL {
            for locale in [Locale::Turkish, Locale::English] {
                assert!(
                    !translate(locale, key).contains('\u{2014}'),
                    "{locale:?} {key:?}"
                );
            }
        }
    }

    #[test]
    fn risk_notice_mentions_terms() {
        let turkish = translate(Locale::Turkish, MessageKey::RiskNoticeBody);
        let english = translate(Locale::English, MessageKey::RiskNoticeBody);
        assert!(turkish.contains("hizmet şart"));
        assert!(english.to_ascii_lowercase().contains("terms of service"));
        assert_ne!(turkish, english);
    }
}
