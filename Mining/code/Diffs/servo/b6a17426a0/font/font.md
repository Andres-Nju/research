File_Code/servo/b6a17426a0/font/font_after.rs --- 1/3 --- Text (14 errors, exceeded DFT_PARSE_ERROR_LIMIT)
24 use values::computed::{Angle, Context, Integer, NonNegative, NonNegativeLength, NonNegativePercentage};                                                   24 use values::computed::{Angle, Context, Integer, NonNegativeLength, NonNegativePercentage};

File_Code/servo/b6a17426a0/font/font_after.rs --- 2/3 --- Text (14 errors, exceeded DFT_PARSE_ERROR_LIMIT)
956         (self.0).0                                                                                                                                       956         self.0.to_animated_value()

File_Code/servo/b6a17426a0/font/font_after.rs --- 3/3 --- Text (14 errors, exceeded DFT_PARSE_ERROR_LIMIT)
961         FontStretch(NonNegative(animated))                                                                                                               961         FontStretch(NonNegativePercentage::from_animated_value(animated))

