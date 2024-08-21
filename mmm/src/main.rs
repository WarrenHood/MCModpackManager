use std::path::PathBuf;

use iced::executor;
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, pick_list, progress_bar, row, scrollable,
    slider, text, text_input, toggler, vertical_rule, vertical_space,
};
use iced::{Alignment, Element, Length, Sandbox, Settings, Theme};
use mcmpmgr::profiles::{self, Profile};

pub fn main() -> iced::Result {
    ManagerGUI::run(Settings::default())
}

#[derive(Default)]
struct ManagerGUI {
    theme: Theme,
    selected_profile: Option<String>,
    userdata: profiles::Data,
    current_view: ManagerView,
    previous_view: ManagerView,
    profile_edit_settings: ProfileSettings,
    profile_save_error: Option<String>,
}

#[derive(Debug, Clone)]
/// The current application view
enum ManagerView {
    ProfileSelect,
    ProfileView { profile: String },
    AddProfile,
    EditProfile { profile: String },
}

#[derive(Debug, Clone)]
/// The current application view
struct ProfileSettings {
    name: String,
    mods_dir: Option<PathBuf>,
    pack_source: String,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),
            mods_dir: Default::default(),
            pack_source: Default::default(),
        }
    }
}

impl Default for ManagerView {
    fn default() -> Self {
        Self::ProfileSelect
    }
}

#[derive(Debug, Clone)]
enum Message {
    SwitchView(ManagerView),
    BrowseModsDir,
    EditProfileName(String),
    EditPackSource(String),
    SaveProfile,
}

impl Sandbox for ManagerGUI {
    type Message = Message;

    fn new() -> Self {
        let mut gui = ManagerGUI::default();
        gui.theme = Theme::GruvboxDark;
        gui
    }

    fn title(&self) -> String {
        String::from("Minecraft Modpack Manager")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SwitchView(view) => {
                match &view {
                    ManagerView::AddProfile => {
                        self.profile_save_error = None;
                        self.profile_edit_settings = ProfileSettings::default();
                    }
                    // TODO: Load profile for EditProfile
                    _ => {}
                };
                self.current_view = view;
            }
            Message::BrowseModsDir => {
                self.profile_edit_settings.mods_dir = rfd::FileDialog::new()
                    .set_title("Select your mods folder")
                    .pick_folder();
            }
            Message::EditProfileName(name) => self.profile_edit_settings.name = name,
            Message::EditPackSource(pack_source) => self.profile_edit_settings.pack_source = pack_source,
            Message::SaveProfile => {
                // TODO: Save profile
                self.profile_save_error = Some(format!(
                    "Unable to save profile '{}'. Saving not implemented",
                    self.profile_edit_settings.name
                ))
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let contents = match &self.current_view {
            ManagerView::ProfileSelect => self.view_profile_select(),
            ManagerView::ProfileView { profile } => self.view_profile_view(&profile),
            ManagerView::AddProfile => self.view_profile_edit("", ManagerView::ProfileSelect),
            ManagerView::EditProfile { profile } => self.view_profile_edit(
                &profile,
                ManagerView::ProfileView {
                    profile: profile.clone(),
                },
            ),
        };

        container(contents)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

impl ManagerGUI {
    fn view_profile_select(&self) -> Element<Message> {
        column![
            text("Profile Select"),
            button("Add profile").on_press(Message::SwitchView(ManagerView::AddProfile))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn view_profile_view(&self, profile_name: &str) -> Element<Message> {
        column![
            text(format!("Modpack Profile: {profile_name}")),
            button("Back").on_press(Message::SwitchView(ManagerView::ProfileSelect)),
            button("Edit profile").on_press(Message::SwitchView(ManagerView::EditProfile {
                profile: profile_name.into()
            }))
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    fn view_profile_edit(
        &self,
        profile_name: &str,
        previous_view: ManagerView,
    ) -> Element<Message> {
        let mut profile_editor = column![
            text("Profile Add/Edit"),
            row![
                "Profile name",
                text_input("Enter your profile name", &self.profile_edit_settings.name)
                    .on_input(Message::EditProfileName)
            ]
            .spacing(5),
            row![
                "Modpack source",
                text_input(
                    "Enter a modpack source. E.g git+https://github.com/WarrenHood/SomeModPack",
                    &self.profile_edit_settings.pack_source
                )
                .on_input(Message::EditPackSource)
            ]
            .spacing(5),
            row![
                "Mods directory",
                text(if let Some(mods_dir) = &self.profile_edit_settings.mods_dir {
                    mods_dir.display().to_string()
                } else {
                    "".into()
                }),
                button("Browse").on_press(Message::BrowseModsDir)
            ]
            .spacing(5),
            row![
                button("Back").on_press(Message::SwitchView(previous_view)),
                button("Save").on_press(Message::SaveProfile)
            ]
            .spacing(10)
        ]
        .spacing(10)
        .padding(20);

        if let Some(save_error) = &self.profile_save_error {
            profile_editor =
                profile_editor.extend([row!["Save error", text(save_error)].spacing(10).into()]);
        };

        profile_editor.into()
    }
}
