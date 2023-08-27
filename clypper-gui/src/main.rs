use std::path::{Path, PathBuf};

use dioxus::prelude::{Scope, Element, rsx, dioxus_elements, use_state, GlobalAttributes, UseState};
use dioxus_desktop::Config;

fn main() {
    dioxus_desktop::launch_with_props(
        app,
        (),
        Config::default(),
    )
}

fn app(ctx: Scope<()>) -> Element{
    let url = use_state(ctx, || "".to_string());
    let cd = std::env::current_dir().unwrap();
    let out_path = use_state(ctx, || cd.to_str().unwrap().to_string());
    let fd = rfd::FileDialog::new()
        .add_filter("video/mp4", &["mp4"])
        .set_directory(cd);

    let queue: &UseState<Vec<PathBuf>> = use_state(ctx, || vec![]);
    ctx.render(
        rsx!{
            header{
                meta{
                    name: "viewport",
                    content: "width=device-width, initial-scale=1"
                }
                link{
                    rel: "stylesheet",
                    href: "https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"
                },
                link{
                    rel: "stylesheet",
                    href: "https://jenil.github.io/bulmaswatch/darkly/bulmaswatch.min.css"
                },
            }
            body{
                div{ class: "columns is-gapless",
                    div{ class: "column is-one-third",
                        section{
                            queue.get().iter().map(|path| rsx!{
                                div{ class: "box",
                                    div{
                                        class: "level",
                                        div{ class: "level-left",
                                            div{ class: "level-item",
                                                p{ class: "title is-5",
                                                    if let Some(name) = path.file_name(){
                                                        name.to_str().unwrap()
                                                    }else{
                                                        "Invalid path..."
                                                    }
                                                },
                                            },
                                        },
                                    },
                                }
                            })
                        },
                    },
                    div{ class: "column",
                        section{  
                            div{ class: "container is-fluid",
                                div{ class: "field",
                                    label{
                                        class: "label has-text-light",
                                        "Clip URL:"
                                    },
                                    div{ class: "control",
                                        input{
                                            class: "input",
                                            oninput: move |evt| url.set(evt.value.clone()),
                                        }
                                    }
                                }

                                div{ class: "field",
                                    label{
                                        class: "label",
                                        "Out path:"
                                    },
                                    div{ class: "controller",
                                        input{
                                            class: "input",
                                            value: "{out_path}",
                                            oninput: move |evt| url.set(evt.value.clone())
                                        }
                                    },
                                    div{ class: "controller",
                                        button{
                                            class: "button",
                                            onclick: move |_| {
                                                let file = fd.clone().save_file();
                                                if let Some(path) = file{
                                                    let path = path.to_str().unwrap().to_string();
                                                    out_path.set(path);
                                                }
                                            },
                                            "Open"
                                        }
                                    },
                                    div{ class: "controller",
                                        button{
                                            class: "button",
                                            onclick: move |_| {

                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    )
}
