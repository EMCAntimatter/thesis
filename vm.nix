# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{ config, pkgs, packages, ... }:

{
  imports =
    [ # Include the results of the hardware scan.
      # ../flake.nix
      # ../flake.nix
#      ./hardware-configuration.nix
    ];

  boot = {
	loader = {
		systemd-boot.enable = true;
		efi.canTouchEfiVariables = true;
	};
  };

  nix.extraOptions = ''
      experimental-features = nix-command flakes
  '';

  # Set your time zone.
  # time.timeZone = "Europe/Amsterdam";

  # Define a user account. Don't forget to set a password with ‘passwd’.
   users.users = {
      ohilyard = {
        isNormalUser = true;
        extraGroups = ["wheel"];
        openssh.authorizedKeys.keyFiles = [
          /home/ohilyard/.ssh/id_ecdsa.pub
        ];
        password = "password"; # Can only be used by someone with serial console access
        packages = with pkgs; [
          vim
        ];
      };
      root = {
        openssh.authorizedKeys.keyFiles = [
          /home/ohilyard/.ssh/id_ecdsa.pub
        ];

        packages = with pkgs; [
          vim
        ];
      };
   };

  environment.systemPackages = with pkgs; [
     vim # Do not forget to add an editor to edit configuration.nix! The Nano editor is also installed by default.
    #  packages.${system}.thesis
  ];

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [
      22
    ];
  };

  services.openssh = {
    kbdInteractiveAuthentication = false;
    passwordAuthentication = false;
  };

  # Copy the NixOS configuration file and link it from the resulting system
  # (/run/current-system/configuration.nix). This is useful in case you
  # accidentally delete configuration.nix.
  system.copySystemConfiguration = true;

  # This value determines the NixOS release from which the default
  # settings for stateful data, like file locations and database versions
  # on your system were taken. It‘s perfectly fine and recommended to leave
  # this value at the release version of the first install of this system.
  # Before changing this value read the documentation for this option
  # (e.g. man configuration.nix or on https://nixos.org/nixos/options.html).
  system.stateVersion = "22.11"; # Did you read the comment?

}

