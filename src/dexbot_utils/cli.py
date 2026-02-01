#!/usr/bin/env python3
"""Command-line interface for dexbot-utils.

Provides CLI commands for exploring robot configurations and utilities.
"""

from pathlib import Path

import typer

from dexbot_utils.configs import get_available_variants, get_robot_config

app = typer.Typer(
    name="dexbot",
    help="DexBot Utilities - Robot configuration and utility tools",
    no_args_is_help=True,
)

cfg_app = typer.Typer(
    name="cfg",
    help="Robot configuration management commands",
    no_args_is_help=True,
)
app.add_typer(cfg_app, name="cfg")


@cfg_app.command("list")
def config_list() -> None:
    """List all available robot configuration variants."""
    variants = get_available_variants()

    typer.echo(f"Available robot configurations: {len(variants)}")
    for variant in sorted(variants):
        typer.echo(f"  - {variant}")


@cfg_app.command("show")
def config_show(
    config: str = typer.Argument(
        ...,
        help="Robot variant name (e.g., vega_1) or path to config file",
    ),
) -> None:
    """Show detailed information for a specific robot configuration.

    CONFIG can be either:
    - A variant name (e.g., vega_1, vega_1_gripper)
    - A path to a configuration file
    """
    # Check if config is a file path
    config_path = Path(config)
    if config_path.exists() and config_path.is_file():
        typer.echo(f"Loading config from file: {config_path}")
        # TODO: Implement file-based config loading if needed
        typer.echo("File-based config loading not yet implemented.", err=True)
        raise typer.Exit(1)

    # Treat as variant name
    variant = config
    try:
        robot_config = get_robot_config(variant)

        typer.echo(f"Robot Configuration: {variant}")
        typer.echo("=" * 70)
        typer.echo(f"Robot Model: {robot_config.robot_model}")
        typer.echo(f"Abbreviation: {robot_config.abbr}")
        typer.echo(f"URDF Path: {robot_config.urdf_path}")

        # List components
        typer.echo(f"\nComponents ({len(robot_config.components)}):")
        for comp_name in sorted(robot_config.components.keys()):
            comp = robot_config.components[comp_name]
            comp_type = type(comp).__name__
            typer.echo(f"  - {comp_name}: {comp_type}")

        # List sensors
        if hasattr(robot_config, "sensors") and robot_config.sensors:
            typer.echo(f"\nSensors ({len(robot_config.sensors)}):")
            for sensor_name in sorted(robot_config.sensors.keys()):
                sensor = robot_config.sensors[sensor_name]
                sensor_type = type(sensor).__name__
                typer.echo(f"  - {sensor_name}: {sensor_type}")

        # List queryables
        if hasattr(robot_config, "querables") and robot_config.querables:
            typer.echo(f"\nQueryables ({len(robot_config.querables)}):")
            for query_name in sorted(robot_config.querables.keys()):
                query_path = robot_config.querables[query_name]
                typer.echo(f"  - {query_name}: {query_path}")

    except ValueError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)


if __name__ == "__main__":
    app()
