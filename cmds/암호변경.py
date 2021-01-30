# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        ob.write('이전암호ː ')
        ob.INTERACTIVE = 0
        ob.input_to(ob.get_oldpass)

