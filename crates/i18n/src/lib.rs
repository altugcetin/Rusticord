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
}

impl MessageKey {
    #[cfg(test)]
    const ALL: [Self; 16] = [
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
}
