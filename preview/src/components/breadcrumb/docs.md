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

To collapse a run of crumbs, put `BreadcrumbEllipsis {}` inside its own `BreadcrumbItem`. It renders a `<span>` (not an `<li>`, so it nests validly in the item) and is hidden from assistive technology, as in shadcn:

```rust
BreadcrumbItem {
    BreadcrumbEllipsis {}
}
```
