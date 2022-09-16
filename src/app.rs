use std::fs::File;
use std::io::{Read};
use eframe::{egui, epi};
use eframe::egui::Ui;
use flash_lso::read::Reader;
use flash_lso::types::{AMFVersion, Element, Header, Lso, Value};
use substring::Substring;

pub enum Message {
    FileOpen(std::path::PathBuf),
    // Other messages
}

pub struct App {
    lso: Lso,

    message_channel: (
        std::sync::mpsc::Sender<Message>,
        std::sync::mpsc::Receiver<Message>,
    )
}

impl Default for App {
    fn default() -> Self {
        Self {
            lso: Lso { header: Header {
                length: 0,
                name: "Not Loaded".to_string(),
                format_version: AMFVersion::AMF0
            }, body: vec![] },
            message_channel: std::sync::mpsc::channel(),
        }
    }
}

impl epi::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        // This is important, otherwise file dialog can hang
        // and messages are not processed
        ctx.request_repaint();

        loop {
            match self.message_channel.1.try_recv() {
                Ok(message) => {
                    let Message::FileOpen(path_buf) = message;
                    let mut file = File::open(path_buf).unwrap();
                    let mut data = Vec::new();

                    let _bytes = file.read_to_end(&mut data);
                    let reader = Reader::default().parse(&data).unwrap();
                    self.lso = reader.1;

                    println!("{:?}", self.lso.header);
                    println!("{:?}", self.lso.body)
                }
                Err(_) => {
                    break;
                }
            }
        }

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Open...").clicked() {
                        let task = rfd::AsyncFileDialog::new()
                            .add_filter("SOL files", &["sol"])
                            .set_directory(std::env::current_dir().unwrap())
                            .pick_file();

                        let message_sender = self.message_channel.0.clone();

                        execute(async move {
                            let file = task.await;

                            if let Some(file) = file {
                                let file_path = std::path::PathBuf::from(file.path());
                                message_sender.send(Message::FileOpen(file_path)).ok();
                            }
                        });
                    }
                    if ui.button("Exit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let header = &mut self.lso.header;
            if header.length != 0 {
                // The central panel the region left after adding TopPanel's and SidePanel's
                ui.heading("Header");

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut header.name);
                });

                ui.horizontal(|ui| {
                    ui.label("SOL/AMF Version:");
                    ui.code(header.format_version);
                });

                ui.horizontal(|ui| {
                    ui.label("Length:");
                    ui.code(header.length);
                });


                ui.heading("Body");
                let amf0 = header.format_version == AMFVersion::AMF0;
                let body = &self.lso.body;

                for element in body {
                    process_element(ui, amf0, element)
                }

                egui::warn_if_debug_build(ui);
            } else {
                ui.heading("No SOL file loaded.");
            }
        });

        /*if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }*/
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn name(&self) -> &str {
        "SOL Editor"
    }
}

fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || {
        futures::executor::block_on(f);
    });
}

fn process_element(ui: &mut Ui, amf0: bool, element: &Element) {
    let mut no_value = false;

    ui.label(&element.name);

    let value = &element.value();

    match value {
        Value::Number(number) => {ui.code(number)},
        Value::Bool(bool) => {ui.code(bool)},
        Value::String(string) => {ui.code(string)},
        Value::Object(list_of_elements, class_definition) => {
            for i in 0..list_of_elements.len() {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    let element1 = &list_of_elements[i];

                    ui.label(&element1.name);
                    process_element(ui, amf0, &element1);
                });
            }

            if class_definition.is_some() {
                let class_definition = &class_definition.as_ref().unwrap();

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if class_definition.name.is_empty() {
                        ui.label("Class Definition: \"\"");
                    } else {
                        ui.label("Class Definition: ");
                    }

                    ui.code(&class_definition.name);
                });

                let static_properties = &class_definition.static_properties;
                let mut static_properties_string = "[".to_owned();

                for static_property in static_properties {
                    static_properties_string.push_str("\"");
                    static_properties_string.push_str(static_property);
                    static_properties_string.push_str("\", ");
                }

                let static_properties_string_length = static_properties_string.len();

                if static_properties_string_length != 1 {
                    static_properties_string = static_properties_string.substring(0, static_properties_string_length - 2).to_string();
                }

                static_properties_string.push_str("]");

                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label("Static Properties: ");
                    ui.code(static_properties_string);
                }).response
            } else {
                ui.code("No Class Definition Found!")
            }
        },
        Value::Null => {
            ui.horizontal(|ui| {
                ui.code("null");
            }).response
        },
        Value::Undefined => {ui.code("undefined")},
        //Value::ECMAArray(Vec<Rc<Value>>, Vec<Element>, u32),
        //Value::StrictArray(Vec<Rc<Value>>),
        //Value::Date(f64, Option<u16>)
        Value::Unsupported => {ui.code("unsupported")},
        Value::XML(string, bool) => {
            ui.horizontal(|ui| {
                ui.code(string);
                ui.code(bool);
            }).response
        }
        //Value::AMF3(Rc<Value>),
        _ => {
            no_value = true;
            ui.code("Couldn't find type.")
        },
    };

    if amf0 && no_value {
        return;
    };

    let mut no_value2 = false;

    match value {
        Value::Integer(integer) => {ui.code(integer);}
        //Value::ByteArray(Vec<u8>) => ,
        //Value::VectorInt(_, _) => ,
        //Value::VectorUInt(_, _) => ,
        //Value::VectorDouble(_, _) => ,
        //Value::VectorObject(_, _, _) => ,
        //Value::Dictionary(_, _) => ,
        //Value::Custom(_, _, _) => ,
        _ => {
            no_value2 = true;
        }
    }

    if !amf0 && no_value && no_value2 {
        ui.code("Couldn't find type.");
    };
}