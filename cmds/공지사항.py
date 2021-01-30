# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        #ob.write('[H[2J') # CLEAR SCREEN
        from lib.io import cat
        cat(ob, 'data/text/notice.txt')

