# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import pickle
        import uuid

        if line == '':
            ob.sendLine('☞ 사용법: [물품번호] 등록취소')
            return
        num = getInt(line)

        mob = ob.env.findObjName('진영')
        if mob == None:
            ob.sendLine('☞ 이곳에서는 불가능해요.')
            return

        try:
            with open("data/config/book.dat", "rb") as fr:
                data = pickle.load(fr)
        except:
            ob.sendLine('☞ 등록 취소 가능한 물품이 없습니다.')
            return
        if num < 1 or num > len(data):
            ob.sendLine('☞ 등록 취소 가능한 물품이 없습니다.')
            return

        itm = data[num - 1]

        if getInt(ob['관리자등급']) < 1000 and ob['이름'] != itm['등록자']:
            ob.sendLine('☞ 자신이 등록한 물품이 아닙니다.')
            return

        del data[num - 1]

        with open("data/config/book.dat", "wb") as fw:
            pickle.dump(data, fw)
        ob.sendLine('☞ 등록 삭제 되었습니다.')
