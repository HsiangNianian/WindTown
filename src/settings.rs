use crate::{
    game::Art,
    i18n::{I18n, Language, Message, tr},
    icons::Icon,
    ui::{self, CREAM, GREEN, INK, MUTED, Menu, Page},
};
use bevy::{prelude::*, ui::FocusPolicy, window::WindowCloseRequested};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    version: u32,
    language: Language,
    server: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            language: Language::English,
            server: crate::network::DEFAULT_SERVER.trim().into(),
            extra: Default::default(),
        }
    }
}

fn read(path: &Path) -> io::Result<Preferences> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Preferences::default()),
        Err(error) => return Err(error),
    };
    let preferences: Preferences = serde_json::from_slice(&bytes)?;
    if preferences.version != 1 {
        return Err(io::Error::other("Unsupported settings version"));
    }
    Ok(preferences)
}

#[derive(Resource)]
pub(crate) struct Settings {
    pub open: bool,
    changed_this_frame: bool,
    dirty: bool,
    preferences: Preferences,
    path: Result<PathBuf, String>,
    notice: Message,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            open: false,
            changed_this_frame: false,
            dirty: true,
            preferences: Preferences::default(),
            path: Err("No settings directory configured".into()),
            notice: tr("settings.instant"),
        }
    }
}

impl Settings {
    pub fn server(&self) -> &str {
        &self.preferences.server
    }

    pub fn remember_server(&mut self, address: &str) {
        if self.preferences.server == address {
            return;
        }
        self.preferences.server = address.to_owned();
        self.persist();
    }

    pub fn load() -> (Self, I18n) {
        let directory = std::env::var_os("YAPSHIRE_SETTINGS_DIR")
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(crate::paths::data_dir);
        Self::from_path(
            directory
                .map(|path| path.join("settings.json"))
                .map_err(|error| error.to_string()),
        )
    }

    fn from_path(path: Result<PathBuf, String>) -> (Self, I18n) {
        let loaded = path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| read(path).map_err(|error| error.to_string()));
        let (mut preferences, notice) = match loaded {
            Ok(preferences) => (preferences, tr("settings.instant")),
            Err(error) => {
                warn!("Cannot load settings: {error}");
                (Preferences::default(), tr("settings.load_failed"))
            }
        };
        preferences.server = crate::network::normalize_server(&preferences.server)
            .unwrap_or_else(|_| crate::network::DEFAULT_SERVER.trim().into());
        let i18n = I18n {
            language: preferences.language,
        };
        (
            Self {
                preferences,
                path,
                notice,
                ..Default::default()
            },
            i18n,
        )
    }

    pub fn blocks_input(&self) -> bool {
        self.open || self.changed_this_frame
    }

    fn choose(&mut self, language: Language, i18n: &mut I18n) {
        if i18n.language != language {
            i18n.language = language;
        }
        self.preferences.language = language;
        self.persist();
    }

    fn persist(&mut self) {
        let result = self.path.as_ref().map_err(Clone::clone).and_then(|path| {
            let bytes =
                serde_json::to_vec_pretty(&self.preferences).map_err(|error| error.to_string())?;
            crate::paths::atomic_write(path, &bytes).map_err(|error| error.to_string())
        });
        self.notice = match result {
            Ok(()) => tr("settings.saved"),
            Err(error) => tr("settings.save_failed").arg("error", error),
        };
        self.dirty = true;
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Close,
    Language(Language),
}
#[derive(Component)]
pub(crate) struct Root;

pub(crate) fn update(
    mut settings: ResMut<Settings>,
    mut i18n: ResMut<I18n>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut buttons: Query<(&Interaction, &Action, &mut BackgroundColor), Changed<Interaction>>,
    mut close: MessageReader<WindowCloseRequested>,
) {
    // A held mouse must not click through the panel after it closes.
    settings.changed_this_frame &= mouse.pressed(MouseButton::Left);
    if settings.open && (keys.just_pressed(KeyCode::Escape) || close.read().next().is_some()) {
        settings.open = false;
        settings.changed_this_frame = true;
        settings.dirty = true;
        return;
    }
    close.clear();
    for (interaction, action, mut background) in &mut buttons {
        let selected = matches!(action, Action::Language(language) if *language == i18n.language);
        *background = BackgroundColor(if selected {
            GREEN
        } else if *interaction != Interaction::None {
            Color::srgb_u8(194, 199, 146)
        } else {
            Color::srgb_u8(229, 220, 189)
        });
        if *interaction != Interaction::Pressed || !mouse.just_pressed(MouseButton::Left) {
            continue;
        }
        match *action {
            Action::Open => settings.open = true,
            Action::Close => settings.open = false,
            Action::Language(language) => settings.choose(language, &mut i18n),
        }
        settings.changed_this_frame = true;
        settings.dirty = true;
        break;
    }
}

fn button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    title: impl Into<Message>,
    action: Action,
    selected: bool,
    node: Node,
) -> Entity {
    let entity = commands
        .spawn((
            Button,
            action,
            node,
            ChildOf(parent),
            BackgroundColor(if selected {
                GREEN
            } else {
                Color::srgb_u8(229, 220, 189)
            }),
            BorderColor::all(if selected { INK } else { MUTED }),
        ))
        .id();
    if action == Action::Open {
        commands.spawn((
            Icon::Settings.image(art),
            Node {
                width: px(16),
                height: px(16),
                ..default()
            },
            FocusPolicy::Pass,
            ChildOf(entity),
        ));
    }
    ui::label(
        commands,
        entity,
        art,
        title,
        18.0,
        if selected { CREAM } else { INK },
    );
    entity
}

pub(crate) fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    i18n: Res<I18n>,
    menu: Res<Menu>,
    art: Res<Art>,
    roots: Query<Entity, With<Root>>,
    mut last_page: Local<Option<Page>>,
) {
    if !settings.dirty && *last_page == Some(menu.page) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    settings.dirty = false;
    *last_page = Some(menu.page);
    let root = commands
        .spawn((
            Root,
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            FocusPolicy::Pass,
        ))
        .id();
    if !settings.open {
        let (bottom, height) = if menu.page == Page::Editor {
            (42, 30)
        } else {
            (80, 42)
        };
        button(
            &mut commands,
            root,
            &art,
            tr("settings.button"),
            Action::Open,
            false,
            Node {
                position_type: PositionType::Absolute,
                right: px(36),
                bottom: px(bottom),
                width: px(156),
                height: px(height),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: px(8),
                border: UiRect::all(px(2)),
                ..default()
            },
        );
        return;
    }
    commands.entity(root).insert((
        BackgroundColor(Color::srgba_u8(24, 42, 40, 210)),
        FocusPolicy::Block,
    ));
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(408),
                top: px(236),
                width: px(624),
                padding: UiRect::all(px(28)),
                flex_direction: FlexDirection::Column,
                row_gap: px(18),
                border: UiRect::all(px(3)),
                ..default()
            },
            BackgroundColor(CREAM),
            BorderColor::all(MUTED),
            ChildOf(root),
        ))
        .id();
    let title = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(16),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    commands.spawn((
        Icon::Settings.image(&art),
        Node {
            width: px(32),
            height: px(32),
            ..default()
        },
        ChildOf(title),
    ));
    ui::label(&mut commands, title, &art, tr("settings.title"), 32.0, INK);
    ui::label(&mut commands, panel, &art, tr("settings.hint"), 18.0, MUTED);
    let heading = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    commands.spawn((
        Icon::Language.image(&art),
        Node {
            width: px(16),
            height: px(16),
            ..default()
        },
        ChildOf(heading),
    ));
    ui::label(
        &mut commands,
        heading,
        &art,
        tr("settings.language"),
        18.0,
        INK,
    );
    let choices = commands
        .spawn((
            Node {
                column_gap: px(12),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for language in Language::ALL {
        button(
            &mut commands,
            choices,
            &art,
            language.native_name(),
            Action::Language(language),
            language == i18n.language,
            Node {
                width: px(272),
                height: px(58),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        );
    }
    ui::label(
        &mut commands,
        panel,
        &art,
        settings.notice.clone(),
        16.0,
        GREEN,
    );
    button(
        &mut commands,
        panel,
        &art,
        tr("settings.close"),
        Action::Close,
        false,
        Node {
            width: percent(100),
            height: px(42),
            border: UiRect::all(px(2)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_settings_blocks_the_same_click_until_the_mouse_is_released() {
        let mut app = App::new();
        app.init_resource::<Settings>()
            .init_resource::<I18n>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<WindowCloseRequested>()
            .add_systems(Update, update);
        app.world_mut().resource_mut::<Settings>().open = true;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut()
            .spawn((Action::Close, Interaction::Pressed, BackgroundColor(CREAM)));
        app.update();
        assert!(!app.world().resource::<Settings>().open);
        assert!(app.world().resource::<Settings>().blocks_input());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Settings>().blocks_input());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<Settings>().blocks_input());
    }

    #[test]
    fn language_survives_restart_and_unknown_preferences_are_preserved() {
        let dir = std::env::temp_dir().join(format!("yapshire-settings-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, br#"{"version":1,"language":"en","volume":0.8}"#).unwrap();
        let (mut settings, mut i18n) = Settings::from_path(Ok(path.clone()));
        settings.choose(Language::Chinese, &mut i18n);
        settings.remember_server("wss://friends.example");
        assert_eq!(i18n.language, Language::Chinese);
        let (saved, reloaded) = Settings::from_path(Ok(path.clone()));
        assert_eq!(reloaded.language, Language::Chinese);
        assert_eq!(saved.server(), "wss://friends.example");
        assert!(!std::fs::read_to_string(&path).unwrap().contains("password"));
        assert_eq!(read(&path).unwrap().extra["volume"], 0.8);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_preferences_and_write_failures_keep_the_game_usable() {
        let dir = std::env::temp_dir().join(format!("yapshire-settings-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        for content in [
            "{broken",
            r#"{"language":"unsupported"}"#,
            r#"{"version":99}"#,
        ] {
            std::fs::write(&path, content).unwrap();
            let (_, i18n) = Settings::from_path(Ok(path.clone()));
            assert_eq!(i18n.language, Language::English);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
        std::fs::write(
            &path,
            br#"{"server":"https://user:password@example.com/path"}"#,
        )
        .unwrap();
        let (settings, _) = Settings::from_path(Ok(path.clone()));
        assert_eq!(settings.server(), crate::network::DEFAULT_SERVER.trim());
        let (mut settings, mut i18n) = Settings::from_path(Ok(dir.clone()));
        settings.choose(Language::Chinese, &mut i18n);
        assert_eq!(i18n.language, Language::Chinese);
        assert!(settings.notice.to_string().contains("Could not save"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
