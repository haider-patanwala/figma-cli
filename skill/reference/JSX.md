# JSX render syntax & pitfalls

`render` / `render-batch` take a JSX string whose root is a `<Frame>`.

## Props

```
Layout    flex="row"|"col"|"none"   gap={16}   p/px/py/pt/pr/pb/pl={n}
          justify="start|center|end|between"    items="start|center|end"
          wrap={true}  rowGap={n}  grow={1}  stretch={true}
Size      w/h={n}   w="fill"|"hug"   minW/maxW/minH/maxH={n}
Position  position="absolute" x={n} y={n}   (give the node a name)
Paint     bg="#fff" | bg="var:primary"   stroke="#000"  strokeWidth={n}  strokeAlign="inside"
          opacity={0.8}  blendMode="multiply"
Corners   rounded={16}  roundedTL/TR/BL/BR={n}  cornerSmoothing={0.6}
Effects   shadow="4px 4px 12px rgba(0,0,0,0.25)"  blur={8}  overflow="hidden"  rotate={45}
```

## Elements

```jsx
<Text size={18} weight="bold" color="#000" font="Inter">Hello</Text>
<Icon name="lucide:home" size={20} color="#fff" />
<Ellipse w={20} h={20} bg="var:primary" />
<Ellipse w={32} h={32} arc={90} arcStart={-90} innerRadius={0.82} bg="var:primary" /> // spinner/pie
<Rectangle w={100} h={4} bg="#ddd" />
```

Weights: thin, extralight, light, regular, medium, semibold, bold, extrabold, black.
Missing fonts fall back to Inter. Icons resolve via the Iconify API (`prefix:name`).

## Common mistakes

| Wrong | Right |
|-------|-------|
| `layout="horizontal"` | `flex="row"` |
| `padding={24}` | `p={24}` |
| `fill="#fff"` | `bg="#fff"` |
| `cornerRadius={12}` | `rounded={12}` |
| `fontSize={18}` | `size={18}` |
| `fontWeight="bold"` | `weight="bold"` |
| `rounded="var:md"` | `rounded={8}` (radius takes a number) |

## Pitfalls

- **Text clipping (most common):** for text to wrap, set `w="fill"` on the parent
  Frame **and** on every `<Text>`.
- **Buttons:** center text with `flex="row" justify="center" items="center"`.
- **No emojis** — use `<Icon>` or shape placeholders; emojis render inconsistently.
- **Toggle switches / edges:** use flex (`justify="end"`/`"between"`), not absolute x/y.
- **Multi-item requests:** N items = N entries in `render-batch`, not one wrapper Frame.

## Variables

`bg="var:name"` binds to a variable. Available shadcn names after
`tokens preset shadcn`: `background`, `foreground`, `card`, `primary`, `secondary`,
`muted`, `accent`, `border`, `input`, `ring`, and their `-foreground` variants.
Pin to a specific collection with `--collection <name>` or `var:collection:name`.
