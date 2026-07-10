{ pkgs, flake, ... }:
pkgs.testers.nixosTest {
  name = "horae-e2e";
  nodes.server = { config, ... }: {
    imports = [ flake.nixosModules.horae ];
    services.horae.enable = true;
    services.horae.database.createLocally = true;
    systemd.services.horae.environment.DEV_LOGIN = "1";
    # Put horae on PATH so the test script can call `horae seed`
    environment.systemPackages = [ config.services.horae.package ];
  };
  testScript = ''
    server.start()
    server.wait_for_unit("postgresql.service")
    server.wait_for_unit("horae.service")
    server.wait_for_open_port(3000)

    # Health check
    server.succeed("curl -s http://localhost:3000/health | grep -q ok")

    # Seed data — run as the horae user (DynamicUser in systemd creates it)
    # so the unix socket auth matches the DB owner.
    server.succeed("sudo -u horae DATABASE_URL=postgres:///horae horae seed")

    # Dev login: POST returns 303 redirect — don't use -f (fails on non-2xx)
    status = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' -X POST http://localhost:3000/auth/dev-login"
    ).strip()
    assert status == "303", f"Expected 303 redirect, got: {status}"

    # Full login flow with cookie jar (follow redirect)
    server.succeed(
      "curl -s -c /tmp/cookies.txt -L -X POST http://localhost:3000/auth/dev-login -o /dev/null"
    )

    # Harvest API: list time entries (session-authenticated)
    result = server.succeed(
      "curl -s -b /tmp/cookies.txt http://localhost:3000/harvest/v2/time_entries"
    )
    assert '"time_entries"' in result, f"Expected Harvest envelope, got: {result[:200]}"
    assert '"per_page"' in result, f"Missing pagination field in: {result[:200]}"
  '';
}
