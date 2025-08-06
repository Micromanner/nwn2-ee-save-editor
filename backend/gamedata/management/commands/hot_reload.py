"""
Django management command to control hot reload functionality
"""
from django.core.management.base import BaseCommand
from gamedata.hot_reload import (
    enable_hot_reload, 
    disable_hot_reload, 
    is_hot_reload_enabled,
    hot_reload_manager
)


class Command(BaseCommand):
    help = 'Control hot reload functionality for development'

    def add_arguments(self, parser):
        parser.add_argument(
            'action',
            type=str,
            choices=['start', 'stop', 'status', 'list'],
            help='Action to perform'
        )

    def handle(self, *args, **options):
        action = options['action']
        
        if action == 'start':
            if enable_hot_reload():
                self.stdout.write(
                    self.style.SUCCESS('Hot reload enabled successfully')
                )
                # List watched paths
                watched = hot_reload_manager.get_watched_paths()
                if watched:
                    self.stdout.write('\nWatching directories:')
                    for path in watched:
                        self.stdout.write(f'  - {path}')
            else:
                self.stdout.write(
                    self.style.ERROR('Failed to enable hot reload')
                )
                
        elif action == 'stop':
            if disable_hot_reload():
                self.stdout.write(
                    self.style.SUCCESS('Hot reload disabled successfully')
                )
            else:
                self.stdout.write(
                    self.style.ERROR('Failed to disable hot reload')
                )
                
        elif action == 'status':
            if is_hot_reload_enabled():
                self.stdout.write(
                    self.style.SUCCESS('Hot reload is ENABLED')
                )
                # List watched paths
                watched = hot_reload_manager.get_watched_paths()
                if watched:
                    self.stdout.write(f'\nWatching {len(watched)} directories:')
                    for path in watched:
                        self.stdout.write(f'  - {path}')
            else:
                self.stdout.write(
                    self.style.WARNING('Hot reload is DISABLED')
                )
                
        elif action == 'list':
            watched = hot_reload_manager.get_watched_paths()
            if watched:
                self.stdout.write(f'Watching {len(watched)} directories:')
                for path in watched:
                    self.stdout.write(f'  - {path}')
            else:
                self.stdout.write('No directories being watched')