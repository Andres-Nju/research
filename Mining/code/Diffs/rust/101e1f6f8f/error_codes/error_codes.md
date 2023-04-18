File_Code/rust/101e1f6f8f/error_codes/error_codes_after.rs --- Rust
1623 E0573: r##"                                                                                                                                             1623 E0573: r##"
1624 Something other than a type has been used when one was expected.                                                                                        1624 Something other than a type has been used when one was expected.
1625                                                                                                                                                         1625 
1626 Erroneous code examples:                                                                                                                                1626 Erroneous code examples:
1627                                                                                                                                                         1627 
1628 ```compile_fail,E0573                                                                                                                                   1628 ```compile_fail,E0573
1629 enum Dragon {                                                                                                                                           1629 enum Dragon {
1630     Born,                                                                                                                                               1630     Born,
1631 }                                                                                                                                                       1631 }
1632                                                                                                                                                         1632 
1633 fn oblivion() -> Dragon::Born { // error!                                                                                                               1633 fn oblivion() -> Dragon::Born { // error!
1634     Dragon::Born                                                                                                                                        1634     Dragon::Born
1635 }                                                                                                                                                       1635 }
1636                                                                                                                                                         1636 
1637 const HOBBIT: u32 = 2;                                                                                                                                  1637 const HOBBIT: u32 = 2;
1638 impl HOBBIT {} // error!                                                                                                                                1638 impl HOBBIT {} // error!
1639                                                                                                                                                         1639 
1640 enum Wizard {                                                                                                                                           1640 enum Wizard {
1641     Gandalf,                                                                                                                                            1641     Gandalf,
1642     Saruman,                                                                                                                                            1642     Saruman,
1643 }                                                                                                                                                       1643 }
1644                                                                                                                                                         1644 
1645 trait Isengard {                                                                                                                                        1645 trait Isengard {
1646     fn wizard(_: Wizard::Saruman); // error!                                                                                                            1646     fn wizard(_: Wizard::Saruman); // error!
1647 }                                                                                                                                                       1647 }
1648 ```                                                                                                                                                     1648 ```
1649                                                                                                                                                         1649 
1650 In all these errors, a type was expected. For example, in the first error, if                                                                           1650 In all these errors, a type was expected. For example, in the first error, if
1651 we want to return the `Born` variant from the `Dragon` enum, we must set the                                                                            1651 we want to return the `Born` variant from the `Dragon` enum, we must set the
1652 function to return the enum and not its variant:                                                                                                        1652 function to return the enum and not its variant:
1653                                                                                                                                                         1653 
1654 ```                                                                                                                                                     1654 ```
1655 enum Dragon {                                                                                                                                           1655 enum Dragon {
1656     Born,                                                                                                                                               1656     Born,
1657 }                                                                                                                                                       1657 }
1658                                                                                                                                                         1658 
1659 fn oblivion() -> Dragon { // ok!                                                                                                                        1659 fn oblivion() -> Dragon { // ok!
1660     Dragon::Born                                                                                                                                        1660     Dragon::Born
1661 }                                                                                                                                                       1661 }
1662 ```                                                                                                                                                     1662 ```
1663                                                                                                                                                         1663 
1664 In the second error, you can't implement something on an item, only on types.                                                                           1664 In the second error, you can't implement something on an item, only on types.
1665 We would need to create a new type if we wanted to do something similar:                                                                                1665 We would need to create a new type if we wanted to do something similar:
1666                                                                                                                                                         1666 
1667 ```                                                                                                                                                     1667 ```
1668 struct Hobbit(u32); // we create a new type                                                                                                             1668 struct Hobbit(u32); // we create a new type
1669                                                                                                                                                         1669 
1670 const HOBBIT: Hobbit = Hobbit(2);                                                                                                                       1670 const HOBBIT: Hobbit = Hobbit(2);
1671 impl Hobbit {} // ok!                                                                                                                                   1671 impl Hobbit {} // ok!
1672 ```                                                                                                                                                     1672 ```
1673                                                                                                                                                         1673 
1674 In the third case, we tried to only expect one variant of the `Wizard` enum,                                                                            1674 In the third case, we tried to only expect one variant of the `Wizard` enum,
1675 which is not possible. To make this work, we need to using pattern matching                                                                             1675 which is not possible. To make this work, we need to using pattern matching
1676 over the `Wizard` enum:                                                                                                                                 1676 over the `Wizard` enum:
1677                                                                                                                                                         1677 
1678 ```                                                                                                                                                     1678 ```
1679 enum Wizard {                                                                                                                                           1679 enum Wizard {
1680     Gandalf,                                                                                                                                            1680     Gandalf,
1681     Saruman,                                                                                                                                            1681     Saruman,
1682 }                                                                                                                                                       1682 }
1683                                                                                                                                                         1683 
1684 trait Isengard {                                                                                                                                        1684 trait Isengard {
1685     fn wizard(w: Wizard) { // error!                                                                                                                    1685     fn wizard(w: Wizard) { // ok!
1686         match w {                                                                                                                                       1686         match w {
1687             Wizard::Saruman => {                                                                                                                        1687             Wizard::Saruman => {
1688                 // do something                                                                                                                         1688                 // do something
1689             }                                                                                                                                           1689             }
1690             _ => {} // ignore everything else                                                                                                           1690             _ => {} // ignore everything else
1691         }                                                                                                                                               1691         }
1692     }                                                                                                                                                   1692     }
1693 }                                                                                                                                                       1693 }
1694 ```                                                                                                                                                     1694 ```
1695 "##,                                                                                                                                                    1695 "##,

