# Vanta Extension API (V1)

Vanta features a strictly sandboxed, opt-in Extension API. This allows developers to build new pages and widgets without polluting the core dashboard logic or relying on messy forks.

## How it works

In V1, extensions are compiled directly into the Vanta binary. 
An extension can provide:
1. **Pages**: Full-screen, pre-designed layouts that automatically appear in the top navigation.
2. **Components (Widgets)**: Isolated UI boxes (e.g. `hello_world`) that users can inject into their own custom layouts in `config.toml`.

## Creating an Extension

The reference implementation is the `TemplateExtension` located in `src/extension/template.rs`. 

To create a new extension:
1. Implement the `Component` trait for your widgets.
2. Implement the `Page` trait if you want a dedicated navigation screen.
3. Implement the `Extension` trait to bundle them together and handle configuration.
4. Register your extension in `src/main.rs`.

```rust
use crate::extension::{Extension, ExtensionMetadata, Component, Page};

pub struct MyExtension;

impl Extension for MyExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: "my_ext",
            name: "My Awesome Extension",
            author: "You",
            version: "1.0",
            description: "Adds cool widgets.",
        }
    }
    
    // Optional: Return components users can use in config.toml
    fn components(&self) -> Vec<Box<dyn Component>> {
        vec![Box::new(MyWidget)]
    }
}
```

## Official Community Integrations

To keep the core binary blazing fast, we recommend keeping standard Vanta slim. 
Large community extensions will be housed in the **vanta-integrations** repository (coming soon), where users can mix and match the crates they want to compile into their personalized Vanta builds.
