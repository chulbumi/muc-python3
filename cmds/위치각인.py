# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 위치각인')
            return
        if line == '비학천룡':
            n, guard = self.getGuardNum(ob)
            if n != 0 and guard[0]['이름'] != '비학천룡':
                ob.sendLine('☞ 비학천룡이 없습니다.')
                return
            ob['위치각인'] = ob.env.index
            ob.sendLine('☞ 현재 위치가 각인되었습니다.')
        else:
            ob.sendLine('☞ 어디에 각인하시려구요?')
            
    def getGuardNum(self, ob):
        n = 0
        guard = []
        for obj in ob.objs:
            if obj['종류'] == '호위':
                n += 1
                guard.append(obj)
        return n, guard
