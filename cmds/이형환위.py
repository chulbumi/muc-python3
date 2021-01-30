# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 위치이동')
            return
        if line == '비학천룡':
            n, guard = self.getGuardNum(ob)
            if n != 0 and guard[0]['이름'] != '비학천룡':
                ob.sendLine('☞ 비학천룡이 없습니다.')
                return
            index = ob['위치각인']
            if index == '':
                ob.sendLine('☞ 각인된 위치가 없습니다.')
                return
            room = getRoom(index)
            if room == None:
                ob.sendLine('* 이동 실패!!!')
                return
        
            if room == ob.env:
                ob.sendLine('☞ 같은 자리에요. ^^')
                return
            
            ob.clearTarget()
        
            ob.enterRoom(room, '소환', '소환')
        else:
            ob.sendLine('☞ 어디로 이동하시려구요?')
            
    def getGuardNum(self, ob):
        n = 0
        guard = []
        for obj in ob.objs:
            if obj['종류'] == '호위':
                n += 1
                guard.append(obj)
        return n, guard
