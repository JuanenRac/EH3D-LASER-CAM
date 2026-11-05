# Cutting and engraving from a drawing: the settings file

`phoenix-laser cut`, `phoenix-laser vector`, `phoenix-laser layers` and the window read an SVG or a DXF drawing and make the G-code of a laser from it. Everything that is tuned lives in one plain text
file, given with `--config FILE` (or chosen in the window with "Load settings", or named by `PHOENIX_LASER_CONFIG`, or `phoenix-laser.cfg` in the folder the window runs in). The file in
`examples/plate.cfg` shows every kind of setting at work.

Nothing is guessed: a line that is not a setting, a key that is not known and a value outside its range are errors that name the line, and a program is never written from a file that has one.
The options of the command line (`--power`, `--speed`, `--kerf`...) are applied over the file, to every layer.

## The file

```text
# a note (also ; at the start of a line)
[section]
key = value
[layer NAME]
key = value
```

Sections: `[limits]`, `[read]`, `[order]`, `[offset]`, `[hatch]`, `[machine]`, `[defaults]` and one `[layer NAME]` for each layer or colour. `[defaults]` is what a layer gets that the file
does not name, and also what every `[layer NAME]` starts from (wherever in the file `[defaults]` is written). The switches are `on` and `off`.

## What a layer is

The shapes of a drawing are grouped, and each group has its own settings:

* **an SVG:** by colour, `#rrggbb` in lower case: the stroke, or the fill of a shape that has no stroke (names such as `red` and `rgb(255, 0, 0)` are turned into `#ff0000`). A shape with neither
  has no colour and is not drawn. A shape without a style is black, `#000000`. A group of Inkscape that is a layer gives its name to the shapes inside it, which is used when a shape has no colour.
* **a DXF:** by layer name, in lower case. The colour of the entity is kept but not used unless `group_by = color`.
* `group_by` (in `[read]`) is `auto` (what is said above), `layer` or `color`.

`phoenix-laser layers DRAWING` lists the groups with their size and what would be done with each.

## What the program reads

| | SVG | DXF (text form) |
|---|---|---|
| Lines and curves | `path` (`M L H V C S Q T A Z`, absolute and relative), `rect` (with round corners), `circle`, `ellipse`, `line`, `polyline`, `polygon` | `LINE`, `LWPOLYLINE` and `POLYLINE` with bulges, `ARC`, `CIRCLE`, `SPLINE` |
| Placing | `transform` (`translate scale rotate skewX skewY matrix`), groups, `viewBox` | the extrusion of an arc, a circle or a light polyline pointing down |
| Units | `mm cm in pt pc px`; a `px` is `25.4 / dpi` mm (`dpi` is 96 unless the file has a size and a view box) | `$INSUNITS` (inch, foot, mile, millimetre, centimetre, metre, mil, yard, micrometre, decimetre); a drawing with no unit uses `dxf_unit_mm` |
| Never read | scripts, images, `use`, text, styles with selectors, filters, gradients, masks, and any entity declared in the file (an external entity or a declared one is an error) | blocks and inserts, text, hatches, dimensions, ellipses, meshes, binary files |

What is left out is listed as a note (in the window and on the standard error of the commands). Curves become straight pieces that are never farther than `tolerance_mm` from the true curve.

## Order of the work

1. The layers that are `engrave` are done first, then the ones that are `cut`; inside each kind by their `order` (the lowest first) and then by name.
2. Open paths whose ends are closer than `join_tolerance_mm` are joined into one path; one that comes back to its start becomes a closed contour.
3. A contour that lies inside another one is cut before it (a hole before the piece it is a hole of), and among the orders that respect this the head travels as little as possible with the laser
   off: the nearest next contour, then a 2-opt improvement that never breaks the rule. A closed contour is entered at the vertex nearest to the head; an open one from its nearer end.

## Cutting

For each pass of a `cut` layer the program writes `F` (the speed), moves the focus when `focus_step` is not 0, and cuts every contour with `S` (the power) turned on only while it cuts.

* **Width of the beam (kerf):** with `kerf` more than 0 a closed contour is moved half of it sideways, with round corners where the offset line goes round a corner. `kerf_side`: `auto` puts the
  beam outside the outline of a piece and inside a hole (the odd levels of nesting), so that both keep their size; `outside` and `inside` put it always on one side; `none` follows the line. Open
  paths are never moved. A contour that is too small or too narrow for the beam, or whose offset would have to lose a part of it, is left out with a note: nothing is better than a line that is wrong.
* **Tabs:** `tabs` bridges of `tab_width` mm are left uncut on the outline of a piece, the first at `1 / (tabs + 1)` of its length from its first point, and so on, evenly. Holes have none. A contour
  too short for them (the bridges would be more than half of it) is cut whole, with a note. Bridges are left on every pass.
* **Several passes:** `passes` times, with the focus (the Z axis) at `focus_z_mm + focus_step * pass`; it goes back to `focus_z_mm` after the layer. A negative step goes down.

## Engraving

A closed shape of an `engrave` layer is filled with lines `hatch_spacing` mm apart at `hatch_angle` degrees (the even-odd rule: a shape inside another is a hole), one way and then the other. The
head runs `overscan_mm` past each end of a line so that it is at speed when the laser is on. Open paths are followed once. `passes` and `focus_step` work as for cutting.

## The G-code and its check

The program starts with `G21`, `G90` and `M4 S0` (dynamic power, as the engraving of a picture), moves with `G0 ... S0` while it travels, writes `S` on the first move of every cut, puts the laser off
with `S0` after it and ends with `M5` and `M2`. Before it leaves the program, every line is checked, and a program that fails is never given:

* only the words the program writes are allowed (`G0 G1 G21 G90 M4 M5 M2 X Y Z F S`); a number that is not finite is refused;
* `S` is never more than `s_max` and never less than 0, and a power that rounds to `S0` is refused (it would not cut);
* the laser is never on during a `G0` move or a move of the focus, and a `G1` move with the laser on needs a speed (`F` more than 0);
* no move goes outside of the bed when `bed_x_mm` and `bed_y_mm` are given (the overscan counts);
* the program ends with `M5` and `M2`.

The file written by the commands and by the window has the same header as the one of a picture (generator, units, source with its SHA-256, tools) and is never written over the drawing or
the settings file it is made from.

## Keys

### `[limits]`: what a hostile or broken file may ask for

| Key | Default | Meaning |
|---|---|---|
| `max_file_bytes` | 33554432 | the largest file that is read |
| `max_depth` | 64 | how deep the elements of an SVG may be nested |
| `max_nodes` | 200000 | the most elements (SVG) or entities (DXF) |
| `max_shapes` | 100000 | the most shapes |
| `max_points` | 2000000 | the most points in all the shapes |
| `max_curve_segments` | 2000 | the most pieces one curve is cut into |
| `max_coordinate_mm` | 100000 | a point farther than this from the origin is a mistake of the file; its shape is left out |

### `[read]`

| Key | Default | Meaning |
|---|---|---|
| `dpi` | 96 | pixels in an inch, from 1 to 2400 (an SVG whose size is in pixels) |
| `tolerance_mm` | 0.01 | how far a flattened curve, or a round corner of the offset, may be from the true one, from 0.001 to 1 |
| `dxf_unit_mm` | 1 | the millimetres of a unit of a DXF that does not say its unit |
| `group_by` | auto | `auto`, `layer` or `color` |
| `align` | bbox | `bbox`: the lower left corner of the work (the layers that are not skipped) is the origin, like a picture; `page`: an SVG keeps its page (the lower left corner of the page is the origin when the file has a size), a DXF keeps its own origin |

### `[order]`

| Key | Default | Meaning |
|---|---|---|
| `join_tolerance_mm` | 0.01 | ends closer than this are joined (0 to 1; 0 joins nothing) |
| `join_max_paths` | 5000 | nothing is joined when a layer has more open paths than this |
| `order_max_items` | 20000 | with more contours than this they are cut inside ones first but not in the shortest order |
| `containment_max_items` | 1500 | with more closed contours than this holes are not told from pieces |
| `two_opt` | on | the improvement of the order |
| `two_opt_budget` | 20000000 | the most moves it tries |
| `two_opt_max_items` | 2000 | it is not tried for more contours than this |

### `[offset]` and `[hatch]`

| Key | Default | Meaning |
|---|---|---|
| `max_points` (`[offset]`) | 5000 | a contour with more points than this is not offset (it is left out with a note) |
| `max_lines` (`[hatch]`) | 100000 | a hatch with more lines than this is an error |

### `[machine]`

| Key | Default | Meaning |
|---|---|---|
| `s_max` | 1000 | the spindle value that is full power for the controller (`$30` of GRBL), from 1 to 100000 |
| `overscan_mm` | 2 | the run past the end of a hatch line, from 0 to 50 |
| `rapid_mm_min` | 6000 | the speed of the moves with the laser off, only to estimate the time |
| `origin_x`, `origin_y` | 0 | where the lower left corner of the work goes |
| `focus_z_mm` | 0 | where the focus (Z) is for the first pass |
| `bed_x_mm`, `bed_y_mm` | 0 | the size of the bed; 0 for both means it is not known; the work may not go outside of it |
| `max_passes` | 50 | the most passes a layer may ask for, up to 1000 |

The keys of `[machine]` are `s_max`, `overscan_mm`, `rapid_mm_min`, `origin_x`, `origin_y`, `focus_z_mm`, `bed_x_mm`, `bed_y_mm` and `max_passes`.

### `[defaults]` and `[layer NAME]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `mode` | cut | `cut`, `engrave`, `skip` | what is done with the layer; a skipped layer is not even used to place the work |
| `power` | 50 | more than 0 up to 100 | percent of `s_max` |
| `speed` | 600 | 1 to 30000 | mm/min |
| `passes` | 1 | 1 to `max_passes` | how many times |
| `focus_step` | 0 | -20 to 20 | mm the focus moves on at each pass (the total may not pass 100 mm) |
| `kerf` | 0 | 0 to 5 | width of the beam in mm |
| `kerf_side` | none | `none`, `auto`, `inside`, `outside` | on which side of the line the beam goes |
| `tabs` | 0 | 0 to 100 | bridges left on the outline of a piece |
| `tab_width` | 1 | 0 to 100 | width of a bridge in mm |
| `hatch_angle` | 0 | -360 to 360 | degrees |
| `hatch_spacing` | 0.1 | 0.01 to 5 | mm between the lines |
| `order` | 0 | any whole number | the layers of a kind are done from the lowest |

## Not tried

Nothing here has cut or engraved anything on a machine: the program is tested on a computer with drawings written by hand, and the G-code has to be checked by a person before it is run.
