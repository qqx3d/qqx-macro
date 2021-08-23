use proc_macro::TokenStream;

pub fn callback(name: String, function: TokenStream) -> TokenStream {
    let mut function = function.to_string().trim().to_string();
    const ERR: &'static str = "Cannot set non-fn item as a callback!";
    function.insert_str(function.find('{').expect(ERR) + 1, ("{
        #[qqx::ctor::ctor]
        fn q() {
            qqx::callback::".to_string() + name.as_str() + "(" + &function[(function.find("fn").expect(ERR) + 3)..function.find('(').expect(ERR)] + ")
        }
    }").as_str());
    function.parse().unwrap()
}
