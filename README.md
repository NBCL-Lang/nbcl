<div align="center">
    <img src=".github/NBCL Logo.svg" alt="NBCL Logo" width="100"/>
    <br/>
    <img src=".github/NBCL Text.svg" alt="NBCL"/>
    <p>Node Based Configuration Language</p>

[Website](https://nbcl-lang.github.io) · [Documentation](https://nbcl-lang.github.io/docs)
</div>
<br/>

`nbcl` is a lightweight, declarative configuration DSL mainly designed for 
defining UI components and general purpose embedding. The syntax is designed to be simple,
and thus follows an HCL-inspired Blocky syntax but with the added benifits of
modularity, scripting capabilities, and simplicity.

## Example

```py
print("Hello, World")

Object "server" {
    port    = 3000
    address = "nbcl-lang.github.io"
}
```

## Resources

- Documentation: https://nbcl-lang.github.io/docs
- Playground: https://nbcl-lang.github.io/playground