# GUI function

## GUI

```jspl
GUI(render: Ident) -> Null
```

render draws each pixel.
It takes (x, y, frame, mouseX, mouseY) and returns a color (0xRRGGBB).

x: -256~255
y: -256~255
frame: 0~...
mouse_x: -256~...
mouse_y: -256~...

```jspl
define(
  draw_pixel,
  { x: Int; y: Int; frame: Int; mouse_x: Int; mouse_y: Int },
  Int,
  {
    r = 0
    g = 0
    b = 0
    if(x > 0, r = 255)
    if(y < 0, g = 255)
    if(x + y > 0, b = 255)
    r << 16 + g << 8 + b
  }
)
GUI(draw_pixel)
```
