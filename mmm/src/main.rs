use std::fmt::format;
use std::path::PathBuf;
use std::str::FromStr;

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, pick_list, progress_bar, row, scrollable,
    slider, text, text_input, toggler, vertical_rule, vertical_space, Column,
};
use iced::{executor, Application, Command, Executor};
use iced::{Alignment, Element, Length, Sandbox, Settings, Theme};
use mcmpmgr::profiles::{self, Profile};
use mcmpmgr::providers::DownloadSide;

pub fn main() -> iced::Result {
    ManagerGUI::run(Settings::default())
}

#[derive(Default)]
struct ManagerGUI {
    theme: Theme,
    selected_profile: Option<String>,
    userdata: profiles::Data,
    userdata_load_error: Option<String>,
    current_view: ManagerView,
    previous_view: ManagerView,
    profile_edit_settings: ProfileSettings,
    profile_save_error: Option<String>,
    current_install_status: ProfileInstallStatus
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
    side: DownloadSide,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),
            mods_dir: Default::default(),
            pack_source: Default::default(),
            side: DownloadSide::Client,
        }
    }
}

impl TryFrom<ProfileSettings> for profiles::Profile {
    type Error = String;
    fn try_from(value: ProfileSettings) -> Result<Self, Self::Error> {
        let mods_dir = value
            .mods_dir
            .ok_or(format!("A mods directory is required"))?;
        let pack_source = value.pack_source;
        Ok(profiles::Profile::new(
            &mods_dir,
            profiles::PackSource::from_str(&pack_source)?,
            value.side,
        ))
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
    DeleteProfile(String),
    InstallProfile(String),
    ProfileInstalled(ProfileInstallStatus)
}

#[derive(Debug, Clone)]
enum ProfileInstallStatus {
    NotStarted,
    Installing,
    Success,
    Error(String)
}

impl Default for ProfileInstallStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl Application for ManagerGUI {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut gui = ManagerGUI::default();
        gui.theme = Theme::GruvboxDark;
        let loaded_userdata = profiles::Data::load();

        match loaded_userdata {
            Ok(userdata) => {
                gui.userdata = userdata;
            }
            Err(err) => {
                gui.userdata_load_error = Some(err.to_string());
            }
        };

        (gui, Command::none())
    }

    fn title(&self) -> String {
        String::from("Minecraft Modpack Manager")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchView(view) => {
                self.current_install_status = ProfileInstallStatus::NotStarted;
                match &view {
                    ManagerView::AddProfile => {
                        self.profile_save_error = None;
                        self.profile_edit_settings = ProfileSettings::default();
                    }
                    ManagerView::ProfileSelect => {
                        let loaded_userdata = profiles::Data::load();

                        match loaded_userdata {
                            Ok(userdata) => {
                                self.userdata = userdata;
                                self.userdata_load_error = None;
                            }
                            Err(err) => {
                                self.userdata_load_error = Some(err.to_string());
                            }
                        };
                    }
                    ManagerView::EditProfile { profile } => {
                        let loaded_profile = self.userdata.get_profile(profile);
                        self.profile_edit_settings.name = profile.trim().into();
                        if let Some(loaded_profile) = loaded_profile {
                            self.profile_edit_settings.name = profile.into();
                            self.profile_edit_settings.mods_dir =
                                Some(loaded_profile.mods_folder.clone());
                            self.profile_edit_settings.pack_source =
                                loaded_profile.pack_source.to_string();
                            self.profile_edit_settings.side = loaded_profile.side;
                        } else {
                            eprintln!("Failed to load existing profile data for {profile}");
                        }
                    }
                    _ => {}
                };
                self.current_view = view;
                Command::none()
            }
            Message::BrowseModsDir => {
                self.profile_edit_settings.mods_dir = rfd::FileDialog::new()
                    .set_title("Select your mods folder")
                    .pick_folder();
                Command::none()
            }
            Message::EditProfileName(name) => {
                self.profile_edit_settings.name = name;
                Command::none()
            }
            Message::EditPackSource(pack_source) => {
                self.profile_edit_settings.pack_source = pack_source;
                Command::none()
            }
            Message::SaveProfile => {
                let profile: Result<profiles::Profile, String> =
                    profiles::Profile::try_from(self.profile_edit_settings.clone());

                if let Ok(profile) = profile {
                    if self.profile_edit_settings.name.trim().len() == 0 {
                        self.profile_save_error =
                            format!("Invalid profile name {}", self.profile_edit_settings.name)
                                .into();
                    } else {
                        self.userdata
                            .add_profile(self.profile_edit_settings.name.trim(), profile);
                        let save_result = self.userdata.save();
                        if let Err(err) = save_result {
                            self.profile_save_error =
                                format!("Unable to save profile: {err:#?}").into();
                        } else {
                            self.current_view = ManagerView::ProfileView {
                                profile: self.profile_edit_settings.name.trim().into(),
                            }
                        }
                    }
                } else if let Err(err) = profile {
                    self.profile_save_error = err.into();
                };

                Command::none()
            }
            Message::DeleteProfile(name) => {
                self.userdata.remove_profile(&name);
                let save_result = self.userdata.save();
                if let Err(err) = save_result {
                    self.profile_save_error = Some(err.to_string());
                } else {
                    self.current_view = ManagerView::ProfileSelect;
                }

                Command::none()
            }
            Message::InstallProfile(name) => {
                self.current_install_status = ProfileInstallStatus::Installing;
                let profile_name = name.clone();
                let profile = self.userdata.get_profile(&name).cloned();
                Command::perform(
                    async move {
                        if let Some(profile) = profile {
                            let result = profile.install().await;
                            if let Err(err) = result {
                                ProfileInstallStatus::Error(format!("{}", err))
                            }
                            else {
                                ProfileInstallStatus::Success
                            }
                        }
                        else {
                            ProfileInstallStatus::Error(format!("Profile '{}' doesn't exist", name))
                        }
                    },
                    Message::ProfileInstalled
                )
            },
            Message::ProfileInstalled(result) => {
                self.current_install_status  = result;

                Command::none()
            },
            
        }
    }

    fn view(&self) -> Element<Message> {
        let contents = match &self.current_view {
            ManagerView::ProfileSelect => self.view_profile_select(),
            ManagerView::ProfileView { profile } => self.view_profile_view(&profile),
            ManagerView::AddProfile => self.view_profile_edit("", ManagerView::ProfileSelect, true),
            ManagerView::EditProfile { profile } => self.view_profile_edit(
                &profile,
                ManagerView::ProfileView {
                    profile: profile.clone(),
                },
                false,
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
        let mut profile_select = column![text("Profile Select"),];

        let mut profiles_list: Column<Message> = column!();
        let mut profile_names = self.userdata.get_profile_names();
        profile_names.sort();

        for profile_name in profile_names.iter() {
            profiles_list = profiles_list.push(
                button(text(profile_name))
                    .on_press(Message::SwitchView(ManagerView::ProfileView {
                        profile: profile_name.into(),
                    }))
                    .width(Length::Fill),
            );
        }

        profile_select =
            profile_select.push(profiles_list.align_items(Alignment::Center).spacing(1));
        profile_select = profile_select
            .push(button("Add profile").on_press(Message::SwitchView(ManagerView::AddProfile)));

        scrollable(
            profile_select
                .spacing(10)
                .align_items(Alignment::Center)
                .padding(10),
        )
        .into()
    }

    fn view_profile_view(&self, profile_name: &str) -> Element<Message> {
        let mut profile_view = if let Some(profile) = self.userdata.get_profile(profile_name) {
            column![
                text(format!("Modpack Profile: {profile_name}"))
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
                row![
                    "Modpack source",
                    text_input("Modpack source", &profile.pack_source.to_string()),
                ]
                .spacing(5),
                row![
                    "Mods directory",
                    text_input("Mods directory", &profile.mods_folder.display().to_string()),
                ]
                .spacing(20),
                row!["Mods to download", text(profile.side),].spacing(5),
                button("Install").on_press(Message::InstallProfile(profile_name.into())),
                row![
                    button("Back").on_press(Message::SwitchView(ManagerView::ProfileSelect)),
                    button("Edit profile").on_press(Message::SwitchView(
                        ManagerView::EditProfile {
                            profile: profile_name.into()
                        }
                    )),
                    button("Delete profile").on_press(Message::DeleteProfile(profile_name.into()))
                ]
                .spacing(5)
            ]
        } else {
            column![
                text(format!("Unable to load profile: {profile_name}")),
                button("Back").on_press(Message::SwitchView(ManagerView::ProfileSelect)),
            ]
        };

        if let Some(err) = &self.userdata_load_error {
            profile_view = profile_view.push(text(err));
        }

        match &self.current_install_status {
            ProfileInstallStatus::NotStarted => {},
            ProfileInstallStatus::Installing => {
                profile_view = profile_view.push(text("Installing..."));
            },
            ProfileInstallStatus::Success => {
                profile_view = profile_view.push(text("Installed"));
            },
            ProfileInstallStatus::Error(err) => {
                profile_view = profile_view.push(text(format!("Failed to install profile: {}", err)));
            },
        };        

        profile_view
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .into()
    }

    fn view_profile_edit(
        &self,
        profile_name: &str,
        previous_view: ManagerView,
        can_edit_name: bool,
    ) -> Element<Message> {
        let current_mods_directory_display = match &self.profile_edit_settings.mods_dir {
            Some(mods_dir) => mods_dir.display().to_string(),
            None => String::from(""),
        };
        let mut profile_editor = column![
            text("Profile Add/Edit").horizontal_alignment(iced::alignment::Horizontal::Center),
            row![
                "Profile name",
                if can_edit_name {
                    text_input("Enter your profile name", &self.profile_edit_settings.name)
                        .on_input(Message::EditProfileName)
                } else {
                    text_input("Profile name", &self.profile_edit_settings.name)
                }
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
                text_input(
                    "Browse for your MC instance's mods directory",
                    &current_mods_directory_display
                ),
                button("Browse").on_press(Message::BrowseModsDir)
            ]
            .spacing(5),
            row![
                button("Back").on_press(Message::SwitchView(previous_view)),
                button("Save").on_press(Message::SaveProfile)
            ]
            .spacing(10)
        ]
        .align_items(Alignment::Center)
        .spacing(10)
        .padding(20);

        if let Some(save_error) = &self.profile_save_error {
            profile_editor =
                profile_editor.extend([row!["Save error", text(save_error)].spacing(10).into()]);
        };

        profile_editor.into()
    }
}
