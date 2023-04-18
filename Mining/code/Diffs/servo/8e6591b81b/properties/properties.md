File_Code/servo/8e6591b81b/properties/properties_after.rs --- Text (1172 errors, exceeded DFT_PARSE_ERROR_LIMIT)
508                 static PREF_NAME: [Option< &str>; ${len(data.longhands) + len(data.shorthands)}] = [                                                     508                 static PREF_NAME: [Option< &str>; ${
...                                                                                                                                                          509                     len(data.longhands) + len(data.shorthands) + len(data.all_aliases())
...                                                                                                                                                          510                 }] = [
509                     % for property in data.longhands + data.shorthands:                                                                                  511                     % for property in data.longhands + data.shorthands + data.all_aliases():

