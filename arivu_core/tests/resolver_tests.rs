use arivu_core::resolver::SmartResolver;

#[test]
fn test_biorxiv_patterns() {
    let resolver = SmartResolver::new();

    // URL with biorxiv
    let action = resolver
        .resolve("https://www.biorxiv.org/content/10.1101/2023.12.01.569584v1")
        .unwrap();
    assert_eq!(action.connector, "biorxiv");
    assert_eq!(action.tool, "get_preprint_by_doi");
    assert_eq!(
        action.arguments.get("doi").unwrap(),
        "10.1101/2023.12.01.569584v1"
    );
    assert_eq!(action.arguments.get("server").unwrap(), "biorxiv");

    // URL with medrxiv
    let action = resolver
        .resolve("https://www.medrxiv.org/content/10.1101/2023.12.01.569584v1")
        .unwrap();
    assert_eq!(action.connector, "biorxiv");
    assert_eq!(action.arguments.get("server").unwrap(), "medrxiv");

    // DOI pattern
    let action = resolver
        .resolve("biorxiv:10.1101/2023.12.01.569584")
        .unwrap();
    assert_eq!(action.connector, "biorxiv");
    assert_eq!(
        action.arguments.get("doi").unwrap(),
        "10.1101/2023.12.01.569584"
    );
    assert_eq!(action.arguments.get("server").unwrap(), "biorxiv");
}

#[test]
fn test_rss_patterns() {
    let resolver = SmartResolver::new();

    let action = resolver.resolve("https://example.com/feed.xml").unwrap();
    assert_eq!(action.connector, "rss");
    assert_eq!(action.tool, "get_feed");
    assert_eq!(
        action.arguments.get("url").unwrap(),
        "https://example.com/feed.xml"
    );

    let action = resolver.resolve("https://example.com/blog.rss").unwrap();
    assert_eq!(action.connector, "rss");
}

#[test]
fn test_discord_patterns() {
    let resolver = SmartResolver::new();

    let action = resolver
        .resolve("https://discord.com/channels/1234567890/9876543210")
        .unwrap();
    assert_eq!(action.connector, "discord");
    assert_eq!(action.tool, "read_messages");
    assert_eq!(action.arguments.get("channel_id").unwrap(), "9876543210");
}
