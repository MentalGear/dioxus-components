The Breadcrumb component displays the current page's location within a navigational hierarchy, as a trail of links.

## Component Structure

```rust
Breadcrumb {
    BreadcrumbList {
        BreadcrumbItem {
            BreadcrumbLink { href: "/", "Home" }
        }
        BreadcrumbSeparator {}
        BreadcrumbItem {
            BreadcrumbLink { href: "/components", "Components" }
        }
        BreadcrumbSeparator {}
        BreadcrumbItem {
            // The current page: not a link, marked `aria-current="page"`.
            BreadcrumbPage { "Breadcrumb" }
        }
    }
}
```
