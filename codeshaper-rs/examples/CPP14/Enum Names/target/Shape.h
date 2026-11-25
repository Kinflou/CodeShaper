#pragma once

enum class Shape { Circle, Square };

enum class Size {
    Small,
    /* Medium, */
    Large = 8,
};

struct Figure {
    Shape shape;
    Size size;
};
