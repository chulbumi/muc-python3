# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import pickle

        mob = ob.env.findObjName('진영')
        if mob == None:
            ob.sendLine('☞ 이곳에서는 불가능해요.')
            return

        try:
            with open("data/config/book.dat", "rb") as fr:
                data = pickle.load(fr)
        except:
            ob.sendLine('☞ 대여가능한 품목이 없어요.')
            return

        if len(data) == 0:
            ob.sendLine('☞ 대여가능한 품목이 없어요.')
            return

        #ob.sendLine('☞ 대여 목록')
        c = 1
        p = 0
        
        for item in data:
            if line != '':
                if line != stripANSI(item['이름']):
                    c += 1
                    continue

            if item['대여가능'] == True:
                m = '대여가능'
            else:
                m = '대여중(' + item['대여'] + ')'
            s = stripANSI(item['이름'])
            s1 = ''
            if len(s) < 8:
                s1 = '\t'
            
            msg = str(c) + '\t' + item['이름'] + s1 + '\t(' + item['등록자'] + ')\t' + m
            if getInt(ob['관리자등급']) > 1000:
                msg += '\t' + str(item['고유번호'])
            c += 1
            p += 1
            ob.sendLine(msg)

        if p == 0:
            ob.sendLine('☞ 대여가능한 품목이 없어요.')
