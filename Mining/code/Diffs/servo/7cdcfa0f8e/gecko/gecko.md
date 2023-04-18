File_Code/servo/7cdcfa0f8e/gecko/gecko_after.rs --- 1/2 --- Text (1702 errors, exceeded DFT_PARSE_ERROR_LIMIT)
3726                     self.gecko.mClip.height = bottom.0 - self.gecko.mClip.y;                                                                            3726                     self.gecko.mClip.height = (bottom - Au(self.gecko.mClip.y)).0;

File_Code/servo/7cdcfa0f8e/gecko/gecko_after.rs --- 2/2 --- Text (1702 errors, exceeded DFT_PARSE_ERROR_LIMIT)
3733                     self.gecko.mClip.width = right.0 - self.gecko.mClip.x;                                                                              3733                     self.gecko.mClip.width = (right - Au(self.gecko.mClip.x)).0;

