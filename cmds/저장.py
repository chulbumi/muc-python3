# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        res = ob.save()
        if res == True:
            ob.sendLine('* 저장 되었습니다.')
        else:
            ob.sendLine('* 저장 실패!!!')
