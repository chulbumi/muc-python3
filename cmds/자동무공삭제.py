# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['자동무공'] == '':
            ob.sendLine('☞ 자동무공 : 없음')
            return
        ob['자동무공'] = ''
        ob.sendLine('☞ 자동무공을 삭제하였습니다.')
