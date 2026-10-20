pub fn greeting() -> &'static str {
    "hello from the cargo fixture"
}

#[cfg(test)]
mod tests {
    #[test]
    fn greets() {
        assert_eq!(super::greeting(), "hello from the cargo fixture");
    }
}
