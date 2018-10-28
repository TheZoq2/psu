use hal::digital::{OutputPin, InputPin};

use core::borrow::{BorrowMut, Borrow};

pub struct Keypad<R, C, I, O>
where R: Borrow<[I]>,
      C: BorrowMut<[O]>,
      I: InputPin,
      O: OutputPin,
{
    columns: C,
    rows: R,
    _phantom: ::core::marker::PhantomData<(I, O)>,
}

impl<R, C, I, O> Keypad<R, C, I, O>
where R: Borrow<[I]>,
      C: BorrowMut<[O]>,
      I: InputPin,
      O: OutputPin,
{
    pub fn new(rows: R, columns: C) -> Self {
        Self {
            rows,
            columns,
            _phantom: ::core::marker::PhantomData
        }
    }
    pub fn read_all_coords(&mut self, buffer: &mut [(u8, u8)]) -> usize {
        // Set all the columns to low
        for mut col in self.columns.borrow_mut() {
            col.set_low();
        }

        let mut current_index = 0;

        for (ci, mut col) in self.columns.borrow_mut().into_iter().enumerate() {
            // Set the column to high
            col.set_high();
            for (ri, row) in self.rows.borrow().iter().enumerate() {
                // Read the pins
                if row.is_high() {
                    buffer[current_index] = (ri as u8, ci as u8);
                    current_index += 1;
                }

                if current_index >= buffer.len() {
                    col.set_low();
                    return current_index;
                }
            }
            // Reset the column
            col.set_low();
        }

        current_index
    }

    pub fn read_first_key(&mut self) -> Option<(u8, u8)> {
        let mut buffer = [(0,0)];
        let amount = self.read_all_coords(&mut buffer);
        if amount != 0 {
            Some(buffer[0])
        }
        else {
            None
        }
    }
}



pub fn translate_coordinate((row, col): (u8, u8), translation: &[&[char]]) -> char {
    translation[col as usize][row as usize]
}

