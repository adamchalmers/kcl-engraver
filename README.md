# KCL engraver

Turns PNGs into KCL engravings.

An image like this:

<img width="423" height="423" alt="Full-color pic of Leo" src="https://github.com/user-attachments/assets/22f7e9ae-6aef-4caf-8406-edd6335c9396" />

gets converted to a 2D bitmap like this:

<img width="423" height="423" alt="Black and white, pixelated pic of Leo" src="https://github.com/user-attachments/assets/4722344d-6ec0-4697-b01f-beee84781e9d" />

And then into a 3D model like this:

<img width="1244" height="410" alt="Isometric and heads-on perspective of the 3D model" src="https://github.com/user-attachments/assets/c61de85f-ef5e-4805-aa52-aa5a50811874" />

# Usage

 - Install Rust
 - `cargo install --path .`
 - Get the KCL: `kcl-engraver input_image.png output_program.kcl --block-size 6`
 - Get the black-and-white PNG: `kcl-engraver input_image.png output_image.png --block-size 6`

Note: the smaller the block size, the more detailed the results, but the longer KCL will take to run.

For example, here's block size 2 vs block size 6.

<img width="423" height="423" alt="Block size 2" src="https://github.com/user-attachments/assets/d221b6e2-5a85-4e33-bbd7-854cb4e4b93e" />

<img width="423" height="423" alt="Block size 6" src="https://github.com/user-attachments/assets/0af67a0d-7a76-433b-9c55-f35ab952f261" />

