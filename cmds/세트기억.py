# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import uuid

        cnt = 0
        savedSet = 'SET-' + str(uuid.uuid4())

        for obj in ob.objs:
            if obj.inUse == False:
                continue
            if obj.get('종류') != '방어구' and obj.get('종류') != '무기':
                continue
            cnt += 1
            aclist = []
            react = obj['반응이름']
            if type(react) == str:
                react = [ react ]
            for name in react:
                if name.startswith('SET-'):
                    continue
                aclist.append(name)
            aclist.append(savedSet)
            obj['반응이름'] = aclist
            """
            acline = ''
            for a in aclist:
                 acline += a + '\r\n'
            obj['반응이름'] = acline[:-2]
            """

        ob['세트기억'] = savedSet

        if cnt == 0:
            ob.sendLine('☞ 무엇을 기억하시려구요?.')
            return

        ob.sendLine('☞ 기억 되었습니다.')
