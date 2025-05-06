{ lib
, stdenvNoCC
, group ? "whispering"
}:

stdenvNoCC.mkDerivation {
  pname = "whispering-udev-rules";
  version = "1.0.0";

  src = ./udev;

  dontBuild = true;

  installPhase = ''
    mkdir -p $out/lib/udev/rules.d
    substituteAll ${./udev/99-whispering.rules} $out/lib/udev/rules.d/99-whispering.rules
  '';

  meta = with lib; {
    description = "Udev rules for Whispering";
    homepage = "https://github.com/yourusername/whispering";
    license = licenses.mit;
    platforms = platforms.linux;
    maintainers = with maintainers; [ ];
  };
} 