use syn::Ident;

pub struct IdentCased(pub Ident);

impl From<&Ident> for IdentCased {
    fn from(ident: &Ident) -> Self {
        IdentCased(ident.clone())
    }
}

impl IdentCased {
    pub fn remove_prefix(&self) -> Self {
        let s = self.0.to_string();
        IdentCased(Ident::new(&s[1..], self.0.span()))
    }

    #[allow(dead_code)]
    pub fn to_title_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Title);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }

    #[allow(dead_code)]
    pub fn to_upper_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Upper);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }

    pub fn to_screaming_snake_case(&self) -> Self {
        let converter = convert_case::Converter::new()
            .to_case(convert_case::Case::ScreamingSnake);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }

    pub fn to_pascal_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Pascal);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }

    #[allow(dead_code)]
    pub fn to_upper_flat(&self) -> Self {
        let converter = convert_case::Converter::new()
            .to_case(convert_case::Case::UpperFlat);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }

    #[allow(dead_code)]
    pub fn remove_whitespace(&self) -> Self {
        let s = self.0.to_string().split_whitespace().collect::<String>();
        IdentCased(Ident::new(&s, self.0.span()))
    }
}

impl From<IdentCased> for Ident {
    fn from(ident: IdentCased) -> Self {
        ident.0
    }
}