# GUI

```json
{"GUI": {"pixel_func": "String (Literal)"}}
```

```jspl
GUI(pixel_func)
```

This `pixel_func` specifies the name of the function to be called to draw each pixel in the GUI window.
The function takes the x coordinate, the y coordinate, the current frame number,
and the mouse y coordinate, and mouse x coordinate of the pixel as arguments
and returns an integer value representing the color of the pixel (0xRR_GG_BB).
It is not possible to run the `GUI` more than once.

x: -256~255
y: -256~255
frame: 0~...
mouse_x: -256~...
mouse_y: -256~...

```jspl
define(
  draw_pixel,
  {x: Int; y: Int; frame: Int; mouse_x: Int; mouse_y: Int},
  Int,
  scope(
    {
    r = 0
    g = 0
    b = 0
    if([$x > 0, r = 255])
    if([$y < 0, g = 255])
    if([{$x + $y} > 0, b = 255])
    +(*($r, 256,  256), *($g, 256, 256), $b)
    }
  )
)
GUI(draw_pixel)
```
