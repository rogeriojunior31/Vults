//! The tray icon on Linux: a StatusNotifierItem of our own (KDE Plasma, and other panels that
//! speak it), so a left click reaches the app (the toolkit's tray opens its menu instead) and the
//! icon can change frame without writing files.

use ksni::TrayMethods;

pub use ksni::Icon;

/// A menu entry: what the app calls it, and what the user reads.
#[derive(Clone, Debug)]
pub enum Entry {
    Item {
        id: &'static str,
        label: String,
    },
    /// One of several, the chosen one marked, between separators; picking one sends its id.
    Choice {
        options: Vec<(&'static str, String)>,
        selected: usize,
    },
}

type Activate = Box<dyn Fn(i32, i32) + Send>;
type Pick = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

struct Item {
    id: String,
    title: String,
    icon: Vec<Icon>,
    attention: bool,
    entries: Vec<Entry>,
    activate: Activate,
    pick: Pick,
}

impl ksni::Tray for Item {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.title.clone(),
            ..Default::default()
        }
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    /// Needs-attention makes the panel show the item even when it hides passive ones.
    fn status(&self) -> ksni::Status {
        if self.attention {
            ksni::Status::NeedsAttention
        } else {
            ksni::Status::Active
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }

    fn attention_icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }

    fn activate(&mut self, x: i32, y: i32) {
        (self.activate)(x, y);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut menu = Vec::new();
        // A choice stands between separators; one is added before the entry that follows it.
        let mut after_choice = false;
        for e in &self.entries {
            let pick = self.pick.clone();
            if std::mem::take(&mut after_choice) {
                menu.push(ksni::MenuItem::Separator);
            }
            match e {
                Entry::Item { id, label } => {
                    let id = *id;
                    menu.push(
                        ksni::menu::StandardItem {
                            label: label.clone(),
                            activate: Box::new(move |_: &mut Self| pick(id)),
                            ..Default::default()
                        }
                        .into(),
                    );
                }
                Entry::Choice { options, selected } => {
                    let ids: Vec<&'static str> = options.iter().map(|(id, _)| *id).collect();
                    if !menu.is_empty() {
                        menu.push(ksni::MenuItem::Separator);
                    }
                    menu.push(
                        ksni::menu::RadioGroup {
                            selected: *selected,
                            select: Box::new(move |_: &mut Self, i: usize| {
                                if let Some(id) = ids.get(i) {
                                    pick(id);
                                }
                            }),
                            options: options
                                .iter()
                                .map(|(_, label)| ksni::menu::RadioItem {
                                    label: label.clone(),
                                    ..Default::default()
                                })
                                .collect(),
                        }
                        .into(),
                    );
                    after_choice = true;
                }
            }
        }
        menu
    }
}

/// The live tray item.
pub struct Tray(ksni::Handle<Item>);

impl std::fmt::Debug for Tray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tray")
    }
}

impl Tray {
    /// Shows this frame, at each size it was drawn (the panel picks the one closest to its own,
    /// so nothing is scaled); `attention` asks the panel to call the user.
    pub async fn show(&self, icon: Vec<Icon>, attention: bool) {
        self.0
            .update(move |item| {
                item.icon = icon;
                item.attention = attention;
            })
            .await;
    }

    /// Replaces the menu (a choice moved, an entry came or went).
    pub async fn set_entries(&self, entries: Vec<Entry>) {
        self.0.update(move |item| item.entries = entries).await;
    }
}

/// Puts the item on the panel, now or once the panel is up.
pub async fn spawn(
    id: &str,
    title: &str,
    icon: Vec<Icon>,
    entries: Vec<Entry>,
    activate: impl Fn(i32, i32) + Send + 'static,
    pick: impl Fn(&str) + Send + Sync + 'static,
) -> Result<Tray, String> {
    let item = Item {
        id: id.to_string(),
        title: title.to_string(),
        icon,
        attention: false,
        entries,
        activate: Box::new(activate),
        pick: std::sync::Arc::new(pick),
    };
    // A desktop still starting has no watcher yet: the item registers once one appears.
    item.assume_sni_available(true)
        .spawn()
        .await
        .map(Tray)
        .map_err(|e| e.to_string())
}

/// A PNG as the tray wants it: ARGB32, big-endian. `None` if it is not an 8-bit RGBA PNG.
pub fn icon_from_png(bytes: &[u8]) -> Option<Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let mut data = buf[..info.buffer_size()].to_vec();
    for px in data.as_chunks_mut::<4>().0 {
        px.rotate_right(1);
    }
    Some(Icon {
        width: i32::try_from(info.width).ok()?,
        height: i32::try_from(info.height).ok()?,
        data,
    })
}
