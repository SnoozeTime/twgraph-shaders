# Procedural macro to generate my shader objects

Problem with vulkano-shaders is that it does not
allow me to reload my shaders easily at runtime. The
module is hidden and private. Furthermore, I don't want
to maintain a fork of vulkano to change this and I want
to learn about proc macros, so here we are.
