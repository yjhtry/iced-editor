mod error;
use rfd::AsyncFileDialog;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use error::EditorError;
use iced::{
    executor, font,
    highlighter::{self, Highlighter},
    keyboard, theme,
    widget::{
        button, column, container, horizontal_space, pick_list, row, text, text_editor, tooltip,
        Text,
    },
    Application, Command, Element, Font, Settings, Subscription, Theme,
};
use tokio::fs;

fn main() -> iced::Result {
    let fonts = include_bytes!("../fonts/editor-icons.ttf").as_slice();

    Editor::run(Settings {
        default_font: font::Font::MONOSPACE,
        fonts: vec![fonts.into()],
        ..Settings::default()
    })
}

struct Editor {
    path: Option<PathBuf>,
    content: text_editor::Content,
    error: Option<EditorError>,
    theme: highlighter::Theme,
    is_dirty: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    New,
    Save,
    FileSaved(Result<PathBuf, EditorError>),
    Open,
    FileOpened(Result<(PathBuf, Arc<String>), EditorError>),
    ThemeSelected(highlighter::Theme),
}

impl Application for Editor {
    type Message = Message;
    type Flags = ();
    type Executor = executor::Default;
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                error: None,
                path: None,
                theme: highlighter::Theme::SolarizedDark,
                is_dirty: true,
            },
            Command::perform(load_file(default_file()), Message::FileOpened),
        )
    }

    fn title(&self) -> String {
        String::from("Editor")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::New => {
                self.path = None;
                self.error = None;
                self.is_dirty = true;
                self.content = text_editor::Content::new();
            }
            Message::Save => {
                let contents = self.content.text();

                return Command::perform(
                    save_file(self.path.clone(), contents),
                    Message::FileSaved,
                );
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                self.is_dirty = false;
            }
            Message::FileSaved(Err(err)) => {
                self.error = Some(err);
            }
            Message::Edit(action) => {
                self.is_dirty = self.is_dirty || action.is_edit();
                self.content.perform(action)
            }
            Message::Open => {
                return Command::perform(pick_file(), Message::FileOpened);
            }
            Message::FileOpened(Ok((path, contents))) => {
                self.path = Some(path);
                self.error = None;
                self.is_dirty = false;
                self.content = text_editor::Content::with_text(&contents);
            }
            Message::FileOpened(Err(err)) => {
                self.error = Some(err);
            }
            Message::ThemeSelected(theme) => {
                self.theme = theme;
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        keyboard::on_key_press(|key, modifiers| match key.as_ref() {
            keyboard::Key::Character("s") if modifiers.command() => Some(Message::Save),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let controls = row![
            action(new_icon(), "New file", Some(Message::New)),
            action(open_icon(), "Open file", Some(Message::Open)),
            action(
                save_icon(),
                "Save file",
                self.is_dirty.then_some(Message::Save)
            ),
            horizontal_space(),
            pick_list(
                highlighter::Theme::ALL,
                Some(self.theme),
                Message::ThemeSelected
            )
        ]
        .spacing(10);

        let input = container(
            text_editor(&self.content)
                .on_action(Message::Edit)
                .highlight::<Highlighter>(
                    highlighter::Settings {
                        theme: self.theme,
                        extension: self
                            .path
                            .as_ref()
                            .and_then(|p| p.extension().map(|e| e.to_str()))
                            .flatten()
                            .unwrap_or("rs")
                            .to_string(),
                    },
                    |highlight, _| highlight.to_format(),
                )
                .height(iced::Length::Fill),
        )
        .padding(10);

        let status_bar = {
            let status = if let Some(error) = &self.error {
                text(error.to_string())
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("New file"),
                }
            };

            let position = {
                let (row, col) = self.content.cursor_position();

                format!("{}:{}", row + 1, col + 1)
            };

            row![status, horizontal_space(), Text::new(position)]
        };

        container(column![controls, input, status_bar])
            .padding(10)
            .into()
    }

    fn theme(&self) -> iced::Theme {
        Theme::Dark
    }
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), EditorError> {
    let handle = AsyncFileDialog::new()
        .pick_file()
        .await
        .ok_or(EditorError::PickFileError)?;

    load_file(handle.path().to_owned()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), EditorError> {
    let contents = fs::read_to_string(&path).await?.into();
    Ok((path, contents))
}

async fn save_file(path: Option<PathBuf>, contents: String) -> Result<PathBuf, EditorError> {
    let path = match path {
        Some(path) => path,
        None => AsyncFileDialog::new()
            .set_title("Choose a name...")
            .save_file()
            .await
            .ok_or(EditorError::PickFileError)?
            .path()
            .to_owned(),
    };

    fs::write(&path, contents).await?;

    Ok(path)
}

fn default_file() -> PathBuf {
    PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")))
}

fn action<'a>(
    content: Element<'a, Message>,
    label: &'a str,
    press: Option<Message>,
) -> Element<'a, Message> {
    let is_disabled = press.is_none();

    tooltip(
        button(container(content).width(30).center_x())
            .on_press_maybe(press)
            .padding([5, 10])
            .style(if is_disabled {
                theme::Button::Secondary
            } else {
                theme::Button::Primary
            }),
        label,
        tooltip::Position::FollowCursor,
    )
    .style(theme::Container::Box)
    .into()
}

fn icon<'a>(endpoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("editor-icons");

    text(endpoint).font(ICON_FONT).into()
}

fn new_icon<'a>() -> Element<'a, Message> {
    icon('\u{E800}')
}

fn open_icon<'a>() -> Element<'a, Message> {
    icon('\u{F115}')
}

fn save_icon<'a>() -> Element<'a, Message> {
    icon('\u{E801}')
}
