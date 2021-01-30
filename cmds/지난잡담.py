# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        for msg in ob.chatHistory:
            ob.sendLine(msg)

