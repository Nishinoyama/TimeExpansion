/////////////////////////////////////////////////////////////
// Created by: Synopsys DC Expert(TM) in wire load mode
// Version   : Q-2019.12-SP4
// Date      : Sun Nov 21 22:18:47 2021
/////////////////////////////////////////////////////////////


module b01 ( line1, line2, reset, outp, overflw, clock, test_si, test_so,
        test_se );
  input line1, line2, reset, clock, test_si, test_se;
  output outp, overflw, test_so;
  wire   N41, N42, N43, N44, N45, n5, n7, n8, n9, n10, n11, n12, n13, n14, n15,
         n16, n17, n18, n19, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29,
         n30, n31, n32, n33, n34;
  wire   [2:0] stato;
  assign test_so = stato[2];

  FD2S stato_reg_0_ ( .D(N41), .TI(overflw), .TE(test_se), .CP(clock), .CD(n5),
        .Q(stato[0]), .QN(n13) );
  FD2S stato_reg_1_ ( .D(N42), .TI(n13), .TE(test_se), .CP(clock), .CD(n5),
        .Q(stato[1]), .QN(n11) );
  FD2S stato_reg_2_ ( .D(N43), .TI(n11), .TE(test_se), .CP(clock), .CD(n5),
        .Q(stato[2]), .QN(n10) );
  FD2S overflw_reg ( .D(N45), .TI(outp), .TE(test_se), .CP(clock), .CD(n5),
        .Q(overflw) );
  FD2S outp_reg ( .D(N44), .TI(test_si), .TE(test_se), .CP(clock), .CD(n5),
        .Q(outp) );
  IVI U7 ( .A(reset), .Z(n5) );
  EOI U9 ( .A(n16), .B(n17), .Z(N44) );
  EOI U10 ( .A(n14), .B(line2), .Z(n17) );
  ND2I U11 ( .A(stato[2]), .B(n18), .Z(n16) );
  ND2I U14 ( .A(n15), .B(n13), .Z(n22) );
  ND2I U15 ( .A(line2), .B(n10), .Z(n19) );
  NR2I U17 ( .A(n9), .B(n25), .Z(n24) );
  ND2I U18 ( .A(n12), .B(n10), .Z(n21) );
  NR2I U20 ( .A(line2), .B(line1), .Z(n26) );
  AO1P U22 ( .A(n29), .B(n11), .C(n30), .D(n31), .Z(n28) );
  ND2I U27 ( .A(line1), .B(line2), .Z(n27) );
  ND2I U28 ( .A(stato[1]), .B(n13), .Z(n18) );
  ND2I U29 ( .A(line1), .B(stato[0]), .Z(n32) );
  IVI U30 ( .A(N45), .Z(n7) );
  IVI U31 ( .A(n34), .Z(n8) );
  IVI U32 ( .A(n21), .Z(n9) );
  IVI U33 ( .A(n18), .Z(n12) );
  IVI U34 ( .A(line1), .Z(n14) );
  IVI U35 ( .A(line2), .Z(n15) );
  AN3 U36 ( .A(stato[2]), .B(stato[0]), .C(line2), .Z(n25) );
  AN3 U37 ( .A(stato[2]), .B(n18), .C(line2), .Z(n31) );
  AN3 U38 ( .A(n10), .B(stato[0]), .C(stato[1]), .Z(N45) );
  AO4 U39 ( .A(n15), .B(n32), .C(stato[0]), .D(n33), .Z(n29) );
  AO2 U40 ( .A(n14), .B(n10), .C(line1), .D(n15), .Z(n33) );
  AO3 U41 ( .A(line1), .B(n7), .C(n8), .D(n28), .Z(N41) );
  NR3 U42 ( .A(n32), .B(line2), .C(n11), .Z(n30) );
  AO4 U43 ( .A(n10), .B(n32), .C(n18), .D(n27), .Z(n34) );
  AO3 U44 ( .A(stato[1]), .B(n23), .C(n8), .D(n24), .Z(N42) );
  AO2 U45 ( .A(n26), .B(stato[2]), .C(stato[0]), .D(n27), .Z(n23) );
  AO3 U46 ( .A(n14), .B(n19), .C(n20), .D(n21), .Z(N43) );
  AO3 U47 ( .A(line1), .B(n22), .C(n11), .D(stato[2]), .Z(n20) );
endmodule
