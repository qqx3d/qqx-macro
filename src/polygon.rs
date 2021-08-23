use proc_macro::TokenStream;
use syn::Type;
use quote::ToTokens;

pub fn polygon(params: String, input: TokenStream) -> TokenStream {
    let mut result = String::new();
    let mut pos = None;
    let mut color = None;
    let mut pos_specified = false;
    let mut color_specified = false;

    if let Ok(syn::Item::Struct(s)) = syn::parse(input) {
        let struct_name = s.ident.to_string();

        result.push_str(format!("#[derive(Copy, Clone)]\n{} struct {} {{", s.vis.to_token_stream().to_string(), struct_name).as_str());

        for field in s.fields {
            let name = field.ident.expect("Missing `name` for field").to_string();

            if name == "pos" {
                match field.ty {
                    Type::Path(ty) => {
                        let ty = ty.path.to_token_stream().to_string();
                        if ty.chars().count() == 4 && ty.starts_with("Vec") {
                            let n = ty.chars().nth(3).unwrap().to_digit(10).expect("Wrong size for `pos`") as u8;
                            if n < 1 || n > 4 { panic!("Wrong size for `pos`") }
                            pos = Some(n.to_string());
                            result.push_str(format!("pos: qqx::{} <f32>,", ty).as_str());
                            pos_specified = true;
                        } else {
                            panic!("Wrong type for `pos`")
                        }
                    }
                    _ => panic!("Wrong type for `pos`")
                }
            } else if name == "color" {
                match field.ty {
                    Type::Path(ty) => {
                        if ty.path.to_token_stream().to_string() != "Color" { panic!("Wrong type for `color`") }
                        result.push_str(format!("color: qqx::Color,").as_str());
                        color_specified = true;
                    },
                    _ => panic!("Wrong type for `color`")
                }
            } else {
                panic!("Unknown field name `{}`!", name)
            }
        }

        result.push_str(format!("}}\nqqx::glium::implement_vertex!{{{},", struct_name).as_str());
        if pos_specified { result.push_str("pos,") }
        if color_specified { result.push_str("color,") }
        result.push('}');

        let mut params: Vec <String> = params.split(',').map(|x| {
            let mut x = x.to_string();
            x.retain(|c| !c.is_whitespace());
            x
        }).collect();

        if !pos_specified { polygon_default_check(&mut params, &mut pos, "pos", 3) }
        if !color_specified { polygon_default_check(&mut params, &mut color, "color", 4) }

        let vs = format!("
            #version 140
            {}
            {}
            void main() {{
                gl_Position = {};
                {};
            }}
        ",  if pos_specified { format!("in vec{} pos;\n", pos.as_ref().unwrap()) } else { String::new() },
            if color_specified { "in vec4 color;\nout vec4 f_color;\n" } else { "" },
            if pos_specified {
                let size = pos.as_ref().unwrap().parse().unwrap();
                if size == 4 { String::from("pos") }
                else {
                    let default = ["", "0.", "0.", "1."];
                    let mut s = String::from("vec4(pos,");
                    println!("{}", size);
                    for i in size..4 {
                        s.push_str(format!("{},", default[i]).as_str())
                    }
                    s.push(')');
                    s
                }
            } else { format!("vec4({},1.)", pos.as_ref().unwrap()) },
            if color_specified {
                "f_color = color"
            } else { "" }
        );

        let fs = format!("
            #version 140
            out vec4 color;
            {}
            void main() {{
                color = {};
            }}
        ",  if color_specified { "in vec4 f_color;" } else { "" },
            if color_specified { String::from("f_color") } else { format!("vec4({})", color.unwrap()) }
        );

        result.push_str(format!("
            impl qqx::OnBoundPolygonInit for {} {{
                fn program(dpy: &qqx::glium::Display) -> &'static qqx::glium::Program {{
                    static mut PROGRAM: Option <qqx::glium::Program> = None;
                    unsafe {{
                        if PROGRAM.is_none() {{
                            PROGRAM = Some(qqx::glium::Program::from_source(dpy, \"{}\", \"{}\", None).unwrap())
                        }}
                        PROGRAM.as_ref().unwrap()
                    }}
                }}
            }}
        ", struct_name, vs, fs).as_str())
    } else {
        panic!("`polygon` takes structure as input")
    }

    println!("{}", result);
    result.parse().unwrap()
}

fn polygon_default_check(params: &mut Vec <String>, to: &mut Option <String>, name: &str, num: usize) {
    let mut i = 0;
    while i < params.len() {
        if params[i].starts_with(format!("{}=", name).as_str()) {
            let tmp = params[i][(name.len() + 1)..].to_string().split('|').map(|x| {
                if x.parse::<f32>().is_err() { panic!("Wrong value for `{}`", name) }
                x.to_string()
            }).collect::<Vec <String>>();
            if tmp.len() != num { panic!("Wrong number of arguments for `{}`", name) }
            *to = Some(tmp.join(","));
            params.remove(i);
            break
        }
        i += 1
    }
    if to.is_none() { panic!("Missing default `{}` specifying", name) }
}
