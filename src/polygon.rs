use proc_macro::TokenStream;
use syn::Type;
use quote::ToTokens;

enum Item {
    /// Item is not specified in any way
    No,

    /// Item is specified through macro attribute, "by hand"
    Specified(String),

    /// Item is specified in shader
    Shader(String, bool),

    /// As above, but field does not contain data
    DummyShader(bool),
}

use Item::*;

impl Item {
    pub const fn is_shader(&self) -> bool {
        matches!(self, Self::Shader(_, _) | Self::DummyShader(_))
    }

    pub const fn is_no(&self) -> bool {
        matches!(self, Self::No)
    }

    pub const fn is_uniform(&self) -> bool {
        match self {
            Shader(_, x) => *x,
            DummyShader(x) => *x,
            _ => false
        }
    }

    pub const fn data(&'a self, ) -> &'a String {
        match self {
            Specified(x) => x,
            Shader(x, _) => x,
            _ => unimplemented!()
        }
    }
}

pub fn polygon(params: String, input: TokenStream) -> TokenStream {
    let mut result = String::new();
    let mut pos = No;
    let mut color = No;

    if let Ok(syn::Item::Struct(s)) = syn::parse(input) {
        let struct_name = s.ident.to_string();

        result.push_str(format!("#[derive(Copy, Clone)]\n{} struct {} {{", s.vis.to_token_stream().to_string(), struct_name).as_str());

        for field in s.fields {
            let name = field.ident.expect("Missing `name` for field").to_string();
            let uniform = field.attrs.iter().find(|c| c.to_token_stream().to_string() == "#[mutable]").is_some();

            if name == "pos" {
                match field.ty {
                    Type::Path(ty) => {
                        let mut ty = super::tls::trim(ty.path.to_token_stream().to_string());

                        let size = ty.chars().position(|c| c == '<').expect("Wrong type for `pos`") - 1;
                        let n = ty.chars().nth(size).unwrap().to_digit(10).expect("Wrong size for `pos`") as u8;
                        ty.remove(size);
                        if n < 1 || n > 4 || ty != "qqx::Vec<f32>" { panic!("Wrong type for `pos`") }

                        pos = Shader(n.to_string(), uniform);
                        result.push_str(format!("pos: qqx::Vec{}<f32>,", n).as_str());
                    }
                    _ => panic!("Wrong type for `pos`")
                }
            } else if name == "color" {
                match field.ty {
                    Type::Path(ty) => {
                        let ty = super::tls::trim(ty.path.to_token_stream().to_string());
                        if ty != "qqx::Color" { panic!("Wrong type for `color`") }
                        result.push_str(format!("color: qqx::Vec4 <f32>,").as_str());
                        color = DummyShader(uniform)
                    },
                    _ => panic!("Wrong type for `color`")
                }
            } else {
                panic!("Unknown field name `{}`", name)
            }
        }

        result.push_str(format!("}}\nqqx::glium::implement_vertex!{{{},", struct_name).as_str());
        if pos.is_shader() { result.push_str("pos,") }
        if color.is_shader() { result.push_str("color,") }
        result.push('}');

        let mut params: Vec <String> = params.split(',').map(|x| super::tls::trim(x.to_string())).collect();

        if pos.is_no() { polygon_default_check(&mut params, &mut pos, "pos", 3) }
        if color.is_no() { polygon_default_check(&mut params, &mut color, "color", 4) }

        let mut size = 3;
        let vs = format!("
            #version 140
            {}
            {}
            {}
            void main() {{
                {};
                gl_Position = {};
                {};
            }}
        ",  if pos.is_shader() { format!("in vec{} pos;\n", pos.data()) } else { String::new() },
            if color.is_shader() { "in vec4 color;\nout vec4 f_color;\n" } else { "" },
            if pos.is_uniform() { format!("uniform vec{} pos_u;\n", pos.data()) } else { String::new() },
            if pos.is_uniform() { format!("vec{} pos = pos + pos_u", pos.data()) } else { String::new() },
            if pos.is_shader() {
                size = pos.data().parse().unwrap();
                if size == 4 { String::from("pos") }
                else {
                    let default = ["", "0.0", "0.0", "1.0"];
                    let mut s = String::from("vec4(pos,");
                    for i in size..4 {
                        s.push_str(format!("{},", default[i]).as_str())
                    }
                    s.pop();
                    s.push(')');
                    s
                }
            } else { format!("vec4({},1.)", pos.data()) },
            if color.is_shader() { "f_color = color" } else { "" }
        );

        let fs = format!("
            #version 140
            out vec4 color;
            {}
            {}
            void main() {{
                color = {};
            }}
        ",  if color.is_shader() { "in vec4 f_color;" } else { "" },
            if color.is_uniform() { "uniform vec4 col_u;" } else { "" },
            super::tls::mix_colors(if color.is_shader() { String::from("f_color") } else { format!("vec4({})", color.data()) }, if color.is_uniform() { String::from("col_u") } else { String::new() })
        );

        let uni_t = if pos.is_uniform() && !color.is_uniform() {
            format!("qqx::glium::uniforms::UniformsStorage <'static, qqx::Vec{} <f32>, qqx::glium::uniforms::EmptyUniforms>", size)
        } else if color.is_uniform() && !pos.is_uniform() {
            String::from("qqx::glium::uniforms::UniformsStorage <'static, qqx::Color, qqx::glium::uniforms::EmptyUniforms>")
        } else if pos.is_uniform() && color.is_uniform() {
            format!("qqx::glium::uniforms::UniformsStorage <'static, qqx::Color, qqx::glium::uniforms::UniformsStorage <'static, qqx::Vec{} <f32>, qqx::glium::uniforms::EmptyUniforms>>", size)
        } else {
            String::from("qqx::glium::uniforms::EmptyUniforms")
        };

        let con_t = format!("({} {})",
            if pos.is_uniform() { format!("qqx::Vec{} <f32>,", size) } else { String::new() },
            if color.is_uniform() { "qqx::Color" } else { "" }
        );

        result.push_str(format!("
            impl qqx::BoundPolygonInterface <{}> for {} {{
                const MOVABLE: bool = {};
                const COLORABLE: bool = {};

                type Move = qqx::Vec{} <f32>;
                type Uniform = {};

                fn program(dpy: &qqx::glium::Display) -> &'static qqx::glium::Program {{
                    static mut PROGRAM: Option <qqx::glium::Program> = None;
                    unsafe {{
                        if PROGRAM.is_none() {{
                            PROGRAM = Some(qqx::glium::Program::from_source(dpy, \"{}\", \"{}\", None).unwrap())
                        }}
                        PROGRAM.as_ref().unwrap()
                    }}
                }}

                fn uniforms(u: &{}) -> {} {{
                    {}
                }}

                fn act_pos(u: &mut {}, action: qqx::BoundPolygonInterfaceAction <Self::Move>) {{
                    {}
                }}

                fn act_col(u: &mut {}, action: qqx::BoundPolygonInterfaceAction <qqx::Color>) {{
                    {}
                }}
            }}
        ",  con_t, struct_name,
            pos.is_uniform(),
            color.is_uniform(),
            size, uni_t, vs, fs, con_t, uni_t,
            if !pos.is_uniform() && !color.is_uniform() { uni_t.clone() } else { format!("glium::uniform!{{ {} {} }}", if pos.is_uniform() { "pos_u: u.0," } else { "" }, if color.is_uniform() { format!("col_u: u.{}", if pos.is_uniform() { '1' } else { '0' }) } else { String::new() }) },
            con_t,
            if pos.is_uniform() {"
                match action {
                    qqx::BoundPolygonInterfaceAction::Move(x) => u.0 += x,
                    qqx::BoundPolygonInterfaceAction::Set(x)  => u.0  = x,
                    qqx::BoundPolygonInterfaceAction::Get(x)  => unsafe { *x = u.0 },
                    qqx::BoundPolygonInterfaceAction::Reset   => u.0 = Default::default()
                }
            "} else { "" },
            con_t,
            if color.is_uniform() {
                let idx = if pos.is_uniform() { '1' } else { '0' };
                format!("
                    match action {{
                        qqx::BoundPolygonInterfaceAction::Move(_) => unreachable!(),
                        qqx::BoundPolygonInterfaceAction::Set(x)  => u.{idx} = x,
                        qqx::BoundPolygonInterfaceAction::Get(x)  => unsafe {{ *x = u.{idx} }},
                        qqx::BoundPolygonInterfaceAction::Reset   => u.{idx} = Default::default()
                    }}
                ", idx = idx)
            } else { String::new() }
        ).as_str());

        result.push_str(format!("
            impl {} {{
                {}
                {}
                pub fn new() -> Self {{
                    Self {{
                        {}
                        {}
                    }}
                }}
            }}
        ", struct_name,
            if color.is_shader() {"
                #[inline]
                pub fn color(mut self, color: qqx::Color) -> Self {
                    self.color = color.into();
                    self
                }
            "} else { "" }.to_string(),
            if pos.is_shader() { format!("
                #[inline]
                pub fn pos(mut self, pos: qqx::Vec{} <f32>) -> Self {{
                    self.pos = pos;
                    self
                }}
            ", pos.data().parse::<f32>().unwrap()) } else { String::new() },
            if pos.is_shader() { "pos: Default::default()," } else { "" },
            if color.is_shader() { "color: Default::default()," } else { "" }
        ).as_str());
    } else {
        panic!("`polygon` takes structure as input")
    }
    result.parse().unwrap()
}

fn polygon_default_check(params: &mut Vec <String>, to: &mut Item, name: &str, num: usize) {
    let mut i = 0;
    while i < params.len() {
        if params[i].starts_with(format!("{}=", name).as_str()) {
            let tmp = params[i][(name.len() + 1)..].to_string().split('|').map(|x| {
                if x.parse::<f32>().is_err() { panic!("Wrong value for `{}`", name) }
                x.to_string()
            }).collect::<Vec <String>>();
            if tmp.len() != num { panic!("Wrong number of arguments for `{}`", name) }
            *to = Specified(tmp.join(","));
            params.remove(i);
            break
        }
        i += 1
    }
    if to.is_no() { panic!("Missing default `{}` specifying", name) }
}
