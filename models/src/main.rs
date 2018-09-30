#[macro_use]
extern crate scad;
extern crate scad_util;

use scad::*;
use scad_util::{keyboard, compositions};
use scad_util::constants::{x_axis, y_axis};

qstruct!(Keypad() {
    rows: i32 = 4,
    cols: i32 = 3,
    grid_spacing: f32 = keyboard::SWITCH_SPACING,
    thickness: f32 = 1.5,
    screwhole_offset: f32 = 5.,
    screwhole_diameter: f32 = 2.5,
    screwhead_diameter: f32 = 5.,
});

impl Keypad {
    fn assembly(&self) -> ScadObject {
        scad!(Difference; {
            self.outer(),
            self.grid(),
        })
    }
    fn grid(&self) -> ScadObject {
        let mut result = scad!(Union);

        for row in 0..self.rows {
            for col in 0..self.cols {
                let row = row as f32;
                let col = col as f32;
                let hole = scad!(Translate(
                    vec3(col * self.grid_spacing, row * self.grid_spacing, 0.)
                ); keyboard::mx_switch_hole());
                result.add_child(hole)
            }
        }

        let x_offset = -self.grid_spacing * (self.cols as f32) / 2. + self.grid_spacing / 2.;
        let y_offset = -self.grid_spacing * (self.rows as f32) / 2. + self.grid_spacing / 2.;
        let translated = scad!(Translate(x_axis() * x_offset + y_axis() * y_offset); {
            result
        });

        translated
    }

    fn outer(&self) -> ScadObject {
        let shape = self.objects_at_corners(
            scad!(Cylinder(self.thickness, Diameter(self.screwhead_diameter)))
        );
        let holes = self.objects_at_corners(
            scad!(Cylinder(self.thickness, Diameter(self.screwhole_diameter)))
        );
        scad!(Difference; {
            scad!(Hull; {
                shape
            }),
            holes
        })
    }

    fn objects_at_corners(&self, object: ScadObject) -> ScadObject {
        let cols = self.cols as f32;
        let rows = self.rows as f32;
        let x_size = cols * self.grid_spacing + self.screwhole_offset * 2. - self.grid_spacing / 2.2;
        let y_size = rows * self.grid_spacing + self.screwhole_offset * 2. - self.grid_spacing / 2.2;

        compositions::object_at_corners(x_axis(), y_axis(), x_size, y_size, object)
    }
}


fn main() {
    let mut file = ScadFile::new();
    file.set_detail(10);

    file.add_object(Keypad::new().assembly());

    file.write_to_file("out.scad".into());
}
